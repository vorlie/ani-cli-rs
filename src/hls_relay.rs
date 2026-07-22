use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode, body::Incoming, header, server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use url::Url;

use crate::{AniError, RequestHeaders, Result, StreamLink};

const MAX_PLAYLIST_BYTES: usize = 2 * 1024 * 1024;
const MAX_RESOURCES: usize = 16_384;
const SCAN_LIMIT: usize = 64 * 1024;

#[derive(Clone)]
struct Registered {
    url: Url,
    headers: RequestHeaders,
}

struct State {
    client: Client,
    base: SocketAddr,
    resources: Mutex<HashMap<String, Registered>>,
    tokens_by_url: Mutex<HashMap<String, String>>,
    counter: AtomicU64,
    secret: String,
}

/// A loopback-only HLS relay. Dropping it stops accepting new requests.
pub struct HlsRelay {
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl Drop for HlsRelay {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        // Dropping the join handle detaches the task; the shutdown signal makes
        // its accept loop exit without cancelling an in-flight response.
        let _ = &self.task;
    }
}

/// Starts a relay and returns a stream whose URL points at its loopback endpoint.
pub async fn relay_stream(stream: &StreamLink) -> Result<(HlsRelay, StreamLink)> {
    if !stream.hls {
        return Err(AniError::Input("only HLS streams can be relayed".into()));
    }
    let upstream = validate_upstream(&stream.url)?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let state = Arc::new(State {
        client: Client::builder()
            .redirect(reqwest::redirect::Policy::limited(8))
            .build()?,
        base: address,
        resources: Mutex::new(HashMap::new()),
        tokens_by_url: Mutex::new(HashMap::new()),
        counter: AtomicU64::new(0),
        secret: format!("{}-{}", std::process::id(), unix_nanos()),
    });
    let token = register(&state, upstream, stream.headers.clone())?;
    let server_state = Arc::clone(&state);
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accepted = listener.accept() => {
                    let Ok((socket, _)) = accepted else { break };
                    let state = Arc::clone(&server_state);
                    tokio::spawn(async move {
                        let service = service_fn(move |request| handle(Arc::clone(&state), request));
                        let _ = http1::Builder::new().serve_connection(TokioIo::new(socket), service).await;
                    });
                }
            }
        }
    });
    let mut local = stream.clone();
    local.url = local_url(address, &token);
    local.headers = RequestHeaders::default();
    for track in &mut local.subtitles {
        let url = validate_upstream(&track.url)?;
        let token = register(&state, url, stream.headers.clone())?;
        track.url = local_url(address, &token);
    }
    Ok((
        HlsRelay {
            shutdown: Some(shutdown_tx),
            task,
        },
        local,
    ))
}

async fn handle(
    state: Arc<State>,
    request: Request<Incoming>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let response = match handle_inner(&state, request).await {
        Ok(response) => response,
        Err(error) => response(
            StatusCode::BAD_GATEWAY,
            "text/plain",
            error.to_string().into_bytes(),
        ),
    };
    Ok(response)
}

async fn handle_inner(
    state: &Arc<State>,
    request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>> {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return Ok(response(
            StatusCode::METHOD_NOT_ALLOWED,
            "text/plain",
            b"method not allowed".to_vec(),
        ));
    }
    let token = request
        .uri()
        .path()
        .strip_prefix("/r/")
        .filter(|v| !v.is_empty());
    let Some(token) = token else {
        return Ok(response(
            StatusCode::NOT_FOUND,
            "text/plain",
            b"not found".to_vec(),
        ));
    };
    let registered = state
        .resources
        .lock()
        .expect("relay registry poisoned")
        .get(token)
        .cloned();
    let Some(registered) = registered else {
        return Ok(response(
            StatusCode::FORBIDDEN,
            "text/plain",
            b"invalid relay token".to_vec(),
        ));
    };
    let mut upstream = state
        .client
        .request(request.method().clone(), registered.url.clone());
    if let Some(value) = &registered.headers.referer {
        upstream = upstream.header(header::REFERER, value);
    }
    if let Some(value) = &registered.headers.origin {
        upstream = upstream.header(header::ORIGIN, value);
    }
    for (name, value) in &registered.headers.extra {
        upstream = upstream.header(name, value);
    }
    for name in [
        header::RANGE,
        header::IF_NONE_MATCH,
        header::IF_MODIFIED_SINCE,
    ] {
        if let Some(value) = request.headers().get(&name) {
            upstream = upstream.header(name, value);
        }
    }
    let upstream = upstream.send().await?;
    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    if request.method() == Method::HEAD {
        let mut result = response(status, &content_type, Vec::new());
        copy_upstream_headers(&upstream_headers, result.headers_mut(), true);
        return Ok(result);
    }
    let bytes = upstream.bytes().await?.to_vec();
    let playlist = content_type.contains("mpegurl")
        || registered
            .url
            .path()
            .to_ascii_lowercase()
            .ends_with(".m3u8")
        || bytes.starts_with(b"#EXTM3U");
    if playlist {
        if bytes.len() > MAX_PLAYLIST_BYTES {
            return Err(AniError::Provider(
                "provider playlist exceeds the 2 MiB relay limit".into(),
            ));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| AniError::Provider("provider playlist is not UTF-8".into()))?;
        let rewritten = rewrite_playlist(state, &registered, &text)?;
        return Ok(response(
            status,
            "application/vnd.apple.mpegurl",
            rewritten.into_bytes(),
        ));
    }
    let (bytes, stripped) = strip_png_wrapper(bytes);
    let mut result = response(
        status,
        if stripped {
            "video/mp2t"
        } else {
            &content_type
        },
        bytes,
    );
    copy_upstream_headers(&upstream_headers, result.headers_mut(), false);
    Ok(result)
}

fn rewrite_playlist(state: &Arc<State>, parent: &Registered, body: &str) -> Result<String> {
    let mut output = String::with_capacity(body.len() + 256);
    for line in body.lines() {
        let trimmed = line.trim();
        let rewritten = if trimmed.is_empty() {
            String::new()
        } else if trimmed.starts_with('#') {
            rewrite_uri_attributes(state, parent, line)?
        } else {
            relay_reference(state, parent, trimmed)?
        };
        output.push_str(&rewritten);
        output.push('\n');
    }
    Ok(output)
}

fn rewrite_uri_attributes(state: &Arc<State>, parent: &Registered, line: &str) -> Result<String> {
    let mut output = line.to_owned();
    let mut offset = 0;
    while let Some(found) = output[offset..].find("URI=\"") {
        let start = offset + found + 5;
        let Some(end_rel) = output[start..].find('"') else {
            break;
        };
        let end = start + end_rel;
        let rewritten = relay_reference(state, parent, &output[start..end])?;
        output.replace_range(start..end, &rewritten);
        offset = start + rewritten.len() + 1;
    }
    Ok(output)
}

fn relay_reference(state: &Arc<State>, parent: &Registered, reference: &str) -> Result<String> {
    let resolved = parent
        .url
        .join(reference)
        .map_err(|error| AniError::Provider(format!("invalid playlist URL: {error}")))?;
    validate_url(&resolved)?;
    let token = register(state, resolved, parent.headers.clone())?;
    Ok(local_url(state.base, &token))
}

fn register(state: &Arc<State>, url: Url, headers: RequestHeaders) -> Result<String> {
    validate_url(&url)?;
    let url_key = url.as_str().to_owned();
    if let Some(token) = state
        .tokens_by_url
        .lock()
        .expect("relay URL registry poisoned")
        .get(&url_key)
        .cloned()
    {
        return Ok(token);
    }
    let mut resources = state.resources.lock().expect("relay registry poisoned");
    if resources.len() >= MAX_RESOURCES {
        return Err(AniError::Provider(
            "provider playlist contains too many resources".into(),
        ));
    }
    let count = state.counter.fetch_add(1, Ordering::Relaxed);
    let token =
        hex::encode(Sha256::digest(format!("{}:{count}:{url}", state.secret)))[..32].to_owned();
    resources.insert(token.clone(), Registered { url, headers });
    state
        .tokens_by_url
        .lock()
        .expect("relay URL registry poisoned")
        .insert(url_key, token.clone());
    Ok(token)
}

fn validate_upstream(value: &str) -> Result<Url> {
    let url =
        Url::parse(value).map_err(|error| AniError::Input(format!("invalid HLS URL: {error}")))?;
    validate_url(&url)?;
    Ok(url)
}

fn validate_url(url: &Url) -> Result<()> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AniError::Provider(
            "credential-bearing playlist URLs are not allowed".into(),
        ));
    }
    let loopback = url
        .host_str()
        .is_some_and(|host| host == "127.0.0.1" || host == "localhost" || host == "::1");
    if url.scheme() != "https" && !(cfg!(test) && loopback && url.scheme() == "http") {
        return Err(AniError::Provider("relay resources must use HTTPS".into()));
    }
    Ok(())
}

fn strip_png_wrapper(bytes: Vec<u8>) -> (Vec<u8>, bool) {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return (bytes, false);
    }
    let limit = bytes.len().min(SCAN_LIMIT);
    for offset in 8..limit.saturating_sub(376) {
        if bytes[offset] == 0x47 && bytes[offset + 188] == 0x47 && bytes[offset + 376] == 0x47 {
            return (bytes[offset..].to_vec(), true);
        }
    }
    (bytes, false)
}

fn response(status: StatusCode, content_type: &str, bytes: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(Full::new(Bytes::from(bytes)))
        .expect("valid relay response")
}

fn copy_upstream_headers(
    source: &hyper::HeaderMap,
    destination: &mut hyper::HeaderMap,
    include_length: bool,
) {
    for name in [
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
        header::ETAG,
        header::LAST_MODIFIED,
        header::CACHE_CONTROL,
    ] {
        if let Some(value) = source.get(&name) {
            destination.insert(name, value.clone());
        }
    }
    if include_length && let Some(value) = source.get(header::CONTENT_LENGTH) {
        destination.insert(header::CONTENT_LENGTH, value.clone());
    }
}

fn local_url(address: SocketAddr, token: &str) -> String {
    format!("http://{address}/r/{token}")
}
fn unix_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    #[test]
    fn strips_only_confirmed_png_wrapped_transport_streams() {
        let mut wrapped = b"\x89PNG\r\n\x1a\nnot-really-png".to_vec();
        let offset = wrapped.len();
        wrapped.resize(offset + 377, 0);
        wrapped[offset] = 0x47;
        wrapped[offset + 188] = 0x47;
        wrapped[offset + 376] = 0x47;
        let (result, stripped) = strip_png_wrapper(wrapped);
        assert!(stripped);
        assert_eq!(result[0], 0x47);
    }

    #[test]
    fn leaves_real_png_data_untouched() {
        let value = b"\x89PNG\r\n\x1a\nordinary image".to_vec();
        assert_eq!(strip_png_wrapper(value.clone()), (value, false));
    }

    #[tokio::test]
    async fn rewrites_nested_resources_and_unwraps_segments() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/master.m3u8"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/vnd.apple.mpegurl")
                    .set_body_string("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nmedia.m3u8\n"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/media.m3u8"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\nsegment.png\n",
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/key.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]))
            .mount(&server)
            .await;
        let mut wrapped = b"\x89PNG\r\n\x1a\nwrapper".to_vec();
        let offset = wrapped.len();
        wrapped.resize(offset + 377, 0);
        wrapped[offset] = 0x47;
        wrapped[offset + 188] = 0x47;
        wrapped[offset + 376] = 0x47;
        Mock::given(method("GET"))
            .and(path("/segment.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(wrapped))
            .mount(&server)
            .await;

        let stream = StreamLink {
            url: format!("{}/master.m3u8", server.uri()),
            resolution: "Auto".into(),
            hls: true,
            provider: "MegaPlay".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
            subtitles: vec![],
        };
        let (_relay, local) = relay_stream(&stream).await.unwrap();
        let client = reqwest::Client::new();
        let master = client
            .get(&local.url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let media_url = master
            .lines()
            .find(|line| line.starts_with("http://"))
            .unwrap();
        let media = client
            .get(media_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let segment_url = media
            .lines()
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .unwrap();
        let segment = client.get(segment_url).send().await.unwrap();
        assert_eq!(segment.headers().get("content-type").unwrap(), "video/mp2t");
        assert_eq!(segment.bytes().await.unwrap()[0], 0x47);
        assert!(media.contains("URI=\"http://127.0.0.1:"));
    }

    #[tokio::test]
    async fn forwards_range_and_rejects_unknown_tokens() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/segment.ts"))
            .and(header("range", "bytes=0-2"))
            .respond_with(
                ResponseTemplate::new(206)
                    .insert_header("content-range", "bytes 0-2/10")
                    .set_body_bytes(vec![1, 2, 3]),
            )
            .expect(1)
            .mount(&server)
            .await;
        let stream = StreamLink {
            url: format!("{}/segment.ts", server.uri()),
            resolution: "Auto".into(),
            hls: true,
            provider: "MegaPlay".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
            subtitles: vec![],
        };
        let (_relay, local) = relay_stream(&stream).await.unwrap();
        let client = reqwest::Client::new();
        let ranged = client
            .get(&local.url)
            .header("range", "bytes=0-2")
            .send()
            .await
            .unwrap();
        assert_eq!(ranged.status(), reqwest::StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            ranged.headers().get("content-range").unwrap(),
            "bytes 0-2/10"
        );
        let base = Url::parse(&local.url).unwrap();
        let rejected = client
            .get(base.join("/r/unknown").unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[test]
    fn rejects_non_https_remote_resources() {
        let error = validate_upstream("http://example.com/master.m3u8").unwrap_err();
        assert!(error.to_string().contains("must use HTTPS"));
    }

    #[test]
    fn caps_registered_playlist_resources() {
        let state = Arc::new(State {
            client: Client::new(),
            base: "127.0.0.1:1".parse().unwrap(),
            resources: Mutex::new(HashMap::new()),
            tokens_by_url: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(0),
            secret: "test".into(),
        });
        for index in 0..MAX_RESOURCES {
            register(
                &state,
                Url::parse(&format!("https://kotocdn.site/{index}")).unwrap(),
                RequestHeaders::default(),
            )
            .unwrap();
        }
        let error = register(
            &state,
            Url::parse("https://kotocdn.site/overflow").unwrap(),
            RequestHeaders::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("too many resources"));
    }
}
