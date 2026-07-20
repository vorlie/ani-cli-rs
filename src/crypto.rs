use std::time::{SystemTime, UNIX_EPOCH};

use aes::Aes256;
use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use base64::{Engine, engine::general_purpose::STANDARD};
use ctr::cipher::{KeyIvInit, StreamCipher};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{AniError, Result};

pub(crate) const EPOCH: u64 = 4128;
pub(crate) const BUILD_ID: &str = "12";
pub(crate) const LEGACY_BUILD_ID: &str = "9";
pub(crate) const QUERY_HASH: &str =
    "d405d0edd690624b66baba3068e0edc3ac90f1597d898a1ec8db4e5c43c00fec";
pub(crate) const STATIC_PART_A: &str =
    "b1a9a4d051988f1b1b12dbb747439d9bd64b09ea17835600a7eaa4de87c1ad87";
pub(crate) const STATIC_PART_B: &str = "k7DLdv5SGiuEyGUtcncl5wQOR7r4aenLfDV3AOBKlAU=";
const RESPONSE_FALLBACK_SECRET: &str = "Xot36i3lK3";

#[derive(Clone, Debug)]
pub(crate) struct CryptoMaterial {
    pub epoch: u64,
    pub build_id: String,
    pub key: [u8; 32],
    pub legacy_ctr: bool,
    pub source: String,
    pub part_a: String,
    pub part_b: String,
    pub app_js_url: Option<String>,
    pub api_url: Option<String>,
    pub fetched_at_ms: u64,
    pub expires_at_ms: u64,
    pub error: Option<String>,
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn query_hash(query: &str) -> String {
    hex::encode(Sha256::digest(query.as_bytes()))
}

pub(crate) fn xor_key(mask_hex: &str, part_b: &str) -> Result<[u8; 32]> {
    let mask =
        hex::decode(mask_hex).map_err(|e| AniError::Bootstrap(format!("invalid Part A: {e}")))?;
    let part = STANDARD
        .decode(part_b)
        .map_err(|e| AniError::Bootstrap(format!("invalid Part B: {e}")))?;
    if mask.len() != 32 || part.len() != 32 {
        return Err(AniError::Bootstrap(
            "crypto material is not 32 bytes".into(),
        ));
    }
    let mut key = [0_u8; 32];
    for i in 0..32 {
        key[i] = mask[i] ^ part[i];
    }
    Ok(key)
}

pub(crate) fn fallback_material(error: Option<String>) -> CryptoMaterial {
    let fetched_at_ms = now_ms();
    CryptoMaterial {
        epoch: EPOCH,
        build_id: BUILD_ID.into(),
        key: xor_key(STATIC_PART_A, STATIC_PART_B).expect("bundled AllAnime key must be valid"),
        legacy_ctr: true,
        source: "fallback".into(),
        part_a: STATIC_PART_A.into(),
        part_b: STATIC_PART_B.into(),
        app_js_url: None,
        api_url: None,
        fetched_at_ms,
        expires_at_ms: fetched_at_ms + 5 * 60_000,
        error,
    }
}

pub(crate) fn aa_req(
    material: &CryptoMaterial,
    query_hash: &str,
    timestamp_ms: u64,
) -> Result<String> {
    let ts = timestamp_ms / 300_000 * 300_000;
    let payload =
        json!({"v":1,"ts":ts,"epoch":material.epoch,"buildId":material.build_id,"qh":query_hash});
    let payload = serde_json::to_vec(&payload)?;
    let iv_digest = Sha256::digest(format!(
        "{}:{}:{}:{}",
        material.epoch, material.build_id, query_hash, ts
    ));
    let iv = &iv_digest[..12];
    let cipher = Aes256Gcm::new_from_slice(&material.key)
        .map_err(|e| AniError::Decryption(e.to_string()))?;
    let encrypted = cipher
        .encrypt(iv.into(), payload.as_ref())
        .map_err(|e| AniError::Decryption(format!("aaReq encryption: {e}")))?;
    let mut result = Vec::with_capacity(13 + encrypted.len());
    result.push(1);
    result.extend_from_slice(iv);
    result.extend_from_slice(&encrypted);
    Ok(STANDARD.encode(result))
}

fn decrypt_gcm(key: &[u8; 32], iv: &[u8], ciphertext_and_tag: &[u8]) -> Result<Value> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AniError::Decryption(e.to_string()))?;
    let plaintext = cipher
        .decrypt(iv.into(), ciphertext_and_tag)
        .map_err(|_| AniError::Decryption("AES-GCM authentication failed".into()))?;
    serde_json::from_slice(&plaintext).map_err(Into::into)
}

pub(crate) fn decode_episode_response(raw: &str, material: &CryptoMaterial) -> Result<Value> {
    let parsed: Value = serde_json::from_str(raw)?;
    if episode_sources(&parsed).is_some() {
        return Ok(parsed);
    }
    let encoded = parsed
        .pointer("/data/episode/tobeparsed")
        .or_else(|| parsed.pointer("/data/tobeparsed"))
        .or_else(|| parsed.get("tobeparsed"))
        .and_then(Value::as_str);
    let Some(encoded) = encoded else {
        return Ok(parsed);
    };
    let buffer = STANDARD
        .decode(encoded)
        .map_err(|e| AniError::Decryption(format!("invalid Base64 payload: {e}")))?;
    if buffer.len() <= 29 {
        return Err(AniError::Decryption(
            "encrypted episode payload is too short".into(),
        ));
    }
    let iv = &buffer[1..13];
    let encrypted = &buffer[13..];
    if let Ok(value) = decrypt_gcm(&material.key, iv, encrypted) {
        return Ok(value);
    }
    let fallback_key: [u8; 32] =
        Sha256::digest(format!("{RESPONSE_FALLBACK_SECRET}:v{}", buffer[0])).into();
    if let Ok(value) = decrypt_gcm(&fallback_key, iv, encrypted) {
        return Ok(value);
    }
    if !material.legacy_ctr {
        return Err(AniError::Decryption(
            "episode payload did not authenticate".into(),
        ));
    }

    type Aes256Ctr = ctr::Ctr128BE<Aes256>;
    let ciphertext = &buffer[13..buffer.len() - 16];
    let mut plaintext = ciphertext.to_vec();
    let mut ctr_iv = [0_u8; 16];
    ctr_iv[..12].copy_from_slice(iv);
    ctr_iv[15] = 2;
    let mut cipher = Aes256Ctr::new((&material.key).into(), (&ctr_iv).into());
    cipher.apply_keystream(&mut plaintext);
    serde_json::from_slice(&plaintext)
        .map_err(|e| AniError::Decryption(format!("legacy AES-CTR payload was invalid JSON: {e}")))
}

pub(crate) fn episode_sources(value: &Value) -> Option<&Value> {
    if value.is_array() {
        return Some(value);
    }
    value
        .get("sourceUrls")
        .or_else(|| value.pointer("/episode/sourceUrls"))
        .or_else(|| value.pointer("/data/episode/sourceUrls"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::Aead;

    #[test]
    fn bundled_key_matches_upstream() {
        assert_eq!(
            hex::encode(xor_key(STATIC_PART_A, STATIC_PART_B).unwrap()),
            "22196fa6afca95309fdabe9a3534b87cd2454e50efeabfcbdbdfd3de678b3982"
        );
    }

    #[test]
    fn aa_req_is_deterministic_and_well_formed() {
        let value = aa_req(&fallback_material(None), QUERY_HASH, 1_700_000_123_456).unwrap();
        let decoded = STANDARD.decode(value).unwrap();
        assert_eq!(decoded[0], 1);
        assert!(decoded.len() > 40);
    }

    #[test]
    fn hashes_graphql_queries_exactly() {
        assert_eq!(
            query_hash("query Example { example }"),
            "79a44000e4a0685e7160f01e353732e2fb31a0445a1235317ec8b1bb2120451a"
        );
    }

    #[test]
    fn decrypts_gcm_episode_payload() {
        let material = fallback_material(None);
        let iv = [7_u8; 12];
        let cipher = Aes256Gcm::new_from_slice(&material.key).unwrap();
        let encrypted = cipher
            .encrypt((&iv).into(), br#"{"sourceUrls":[]}"#.as_ref())
            .unwrap();
        let mut bytes = vec![1];
        bytes.extend(iv);
        bytes.extend(encrypted);
        let raw = json!({"data":{"episode":{"tobeparsed":STANDARD.encode(bytes)}}}).to_string();
        assert!(
            episode_sources(&decode_episode_response(&raw, &material).unwrap())
                .unwrap()
                .is_array()
        );
    }
}
