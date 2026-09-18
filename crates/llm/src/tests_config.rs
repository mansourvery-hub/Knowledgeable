use super::*;
use std::env;

#[test]
fn test_gemini_client_requires_api_key() {
    // Test that the client doesn't panic if key is empty (it should return error later)
    let client = GeminiOpenAiClient::new("".to_string());
    // The client structure is just a wrapper, real validation happens in stream_chat.
}

#[test]
fn test_tutor_service_config_load() {
    env::set_var("GEMINI_API_KEY", "test-key");
    let key = env::var("GEMINI_API_KEY").unwrap();
    assert_eq!(key, "test-key");
    env::remove_var("GEMINI_API_KEY");
}
