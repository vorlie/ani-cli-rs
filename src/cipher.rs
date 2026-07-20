use std::{collections::HashMap, path::Path};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{AniError, Result};

const BUILTIN_PAIRS: &str = "79=A;7a=B;7b=C;7c=D;7d=E;7e=F;7f=G;70=H;71=I;72=J;73=K;74=L;75=M;76=N;77=O;68=P;69=Q;6a=R;6b=S;6c=T;6d=U;6e=V;6f=W;60=X;61=Y;62=Z;59=a;5a=b;5b=c;5c=d;5d=e;5e=f;5f=g;50=h;51=i;52=j;53=k;54=l;55=m;56=n;57=o;48=p;49=q;4a=r;4b=s;4c=t;4d=u;4e=v;4f=w;40=x;41=y;42=z;08=0;09=1;0a=2;0b=3;0c=4;0d=5;0e=6;0f=7;00=8;01=9;15=-;16=.;67=_;46=~;02=:;17=/;07=?;1b=#;63=[;65=];78=@;19=!;1c=$;1e=&;10=(;11=);12=*;13=+;14=,;03=;;05==;1d=%";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CipherMapInfo {
    pub source: String,
    pub tag: Option<String>,
    pub generated_at_unix_ms: u64,
    pub entries: usize,
    pub cipher_map: HashMap<String, String>,
}

pub(crate) fn builtin_cipher_map() -> HashMap<String, String> {
    let mut map: HashMap<String, String> = BUILTIN_PAIRS
        .split(';')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            if key.len() == 2 {
                Some((key.into(), value.into()))
            } else {
                None
            }
        })
        .collect();
    map.insert("03".into(), ";".into());
    map
}

pub fn parse_upstream_cipher_map(content: &str) -> Result<HashMap<String, String>> {
    let regex = Regex::new(r"s/\^([0-9a-fA-F]{2})\$/((?:\\.|[^/])*)/").expect("valid cipher regex");
    let mut map = HashMap::new();
    for capture in regex.captures_iter(content) {
        let value = capture[2]
            .replace(r"\/", "/")
            .replace(r"\[", "[")
            .replace(r"\]", "]")
            .replace(r"\(", "(")
            .replace(r"\)", ")")
            .replace(r"\$", "$")
            .replace(r"\\", "\\");
        map.insert(capture[1].to_ascii_lowercase(), value);
    }
    if map.len() < 60 {
        return Err(AniError::Provider(format!(
            "upstream cipher map had only {} entries",
            map.len()
        )));
    }
    Ok(map)
}

pub(crate) async fn load_cached(path: &Path) -> Option<CipherMapInfo> {
    let bytes = tokio::fs::read(path).await.ok()?;
    let info: CipherMapInfo = serde_json::from_slice(&bytes).ok()?;
    (info.cipher_map.len() >= 60).then_some(info)
}

pub(crate) async fn save_cached(path: &Path, info: &CipherMapInfo) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = serde_json::to_vec_pretty(info)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

pub(crate) fn decode_url(encoded: &str, map: &HashMap<String, String>) -> String {
    if !encoded.starts_with("--") {
        return encoded.to_owned();
    }
    encoded.as_bytes()[2..]
        .chunks(2)
        .filter_map(|pair| std::str::from_utf8(pair).ok())
        .filter_map(|pair| map.get(pair))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builtin_map_decodes_url_prefix() {
        assert_eq!(
            decode_url("--504c4c484b17", &builtin_cipher_map()),
            "https/"
        );
    }
    #[test]
    fn parses_sed_map() {
        let input = (0..60)
            .map(|n| format!("s/^{:02x}$/x/g;", n))
            .collect::<String>();
        assert_eq!(parse_upstream_cipher_map(&input).unwrap().len(), 60);
    }
}
