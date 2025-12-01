//! Utility functions for WASM

use chrono::Utc;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Calculate SHA-256 digest
#[wasm_bindgen(js_name = calculateDigest)]
pub fn calculate_digest(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

/// Generate a UUID
#[wasm_bindgen(js_name = generateId)]
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string().replace("-", "")
}

/// Get current timestamp
#[wasm_bindgen(js_name = getCurrentTimestamp)]
pub fn get_current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Simple sleep function for WASM
pub async fn gloo_timers_sleep(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = web_sys::window().unwrap();
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap();
    });
    let _ = JsFuture::from(promise).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_digest() {
        let digest = calculate_digest(b"hello world");
        assert!(digest.starts_with("sha256:"));
    }

    #[test]
    fn test_generate_id() {
        let id = generate_id();
        assert_eq!(id.len(), 32);
    }
}
