use ani_cli::{AllAnimeClient, SearchOptions, TranslationType};
use serde_json::json;
use tempfile::tempdir;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, method, path},
};

#[tokio::test]
async fn parses_search_response_from_graphql() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(body_partial_json(json!({"variables":{"translationType":"sub"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":{"shows":{"edges":[{"_id":"show-1","name":"Example","availableEpisodes":{"sub":12}}]}}
        })))
        .mount(&server).await;
    let client = AllAnimeClient::builder()
        .api_url(format!("{}/api", server.uri()))
        .state_dir(tempdir().unwrap().path())
        .build()
        .unwrap();
    let results = client
        .search("example", TranslationType::Sub)
        .await
        .unwrap();
    assert_eq!(results[0].id, "show-1");
    assert_eq!(results[0].episodes, 12.0);
}

#[tokio::test]
async fn sends_allow_adult_search_option() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(body_partial_json(
            json!({"variables":{"search":{"allowAdult":true}}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":{"shows":{"edges":[]}}
        })))
        .mount(&server)
        .await;
    let client = AllAnimeClient::builder()
        .api_url(format!("{}/api", server.uri()))
        .state_dir(tempdir().unwrap().path())
        .build()
        .unwrap();
    let results = client
        .search_with_options(
            "example",
            TranslationType::Sub,
            SearchOptions { allow_adult: true },
        )
        .await
        .unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn sorts_fractional_episode_values() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":{"show":{"availableEpisodesDetail":{"sub":["10","2.5",1,2]}}}
        })))
        .mount(&server)
        .await;
    let client = AllAnimeClient::builder()
        .api_url(format!("{}/api", server.uri()))
        .state_dir(tempdir().unwrap().path())
        .build()
        .unwrap();
    assert_eq!(
        client.episodes("show", TranslationType::Sub).await.unwrap(),
        vec!["1", "2", "2.5", "10"]
    );
}

#[tokio::test]
async fn falls_back_to_full_episode_query_and_resolves_direct_media() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/bootstrap"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":{"episode":{"sourceUrls":[{"sourceName":"S-mp4","sourceUrl":"https://media.example/video.mp4"}]}}
        }))).mount(&server).await;
    let directory = tempdir().unwrap();
    let client = AllAnimeClient::builder()
        .api_url(format!("{}/api", server.uri()))
        .bootstrap_url(format!("{}/bootstrap", server.uri()))
        .state_dir(directory.path())
        .build()
        .unwrap();
    let streams = client
        .streams("show", "1", TranslationType::Sub)
        .await
        .unwrap();
    assert_eq!(streams[0].provider, "S-mp4");
    assert_eq!(streams[0].url, "https://media.example/video.mp4");
}
