//! Per-turn LLM provider dispatch (M3).
//!
//! The chat adapter resolves which provider serves each turn from the
//! requested model name, independent of the boot default:
//!
//! - `gemini*` → Gemini OpenAI-compatible client (request BYOK key first,
//!   else `GEMINI_API_KEY`).
//! - `gpt*` / `o1*` / `o3*` → OpenAI client (BYOK first, else `OPENAI_API_KEY`).
//! - `local-tutor` → deterministic fake (offline demos and tests).
//! - anything else (or absent) → the boot default (`default_llm`).
//!
//! Removed-for-now (mobile-first, 2026-09-22): the `ollama*` provider
//! branch (`OLLAMA_*` env, advertised model). The generic
//! `OpenAiClient::with_base_url` stays — any future local/mobile relay
//! reuses it. Restoration is one revert.
//!
//! Request keys are turn-scoped: never stored, never logged (QUALITY.md
//! credential rule). A missing key is an explicit error, never a silent
//! fallback to the wrong provider.

use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProvider {
    Gemini,
    OpenAI,
    LocalTutor,
    BootDefault,
}

#[derive(Debug, Clone)]
pub struct LlmPlan {
    pub provider: LlmProvider,
    /// Model string forwarded to the provider request.
    pub model: String,
    /// Resolved credential (request BYOK or server env). `None` for keyless
    /// providers and the boot default.
    pub key: Option<String>,
}

/// Server key material, read once per turn from the environment.
#[derive(Debug, Clone, Default)]
pub struct ProviderKeys {
    pub gemini: Option<String>,
    pub openai: Option<String>,
}

impl ProviderKeys {
    pub fn from_env() -> Self {
        fn present(name: &str) -> Option<String> {
            std::env::var(name).ok().filter(|v| !v.trim().is_empty())
        }
        Self { gemini: present("GEMINI_API_KEY"), openai: present("OPENAI_API_KEY") }
    }
}

/// Models advertised by `GET /api/models`: provider models for configured
/// keys, always plus the offline `local-tutor` fallback.
pub fn advertised_models() -> Vec<String> {
    let keys = ProviderKeys::from_env();
    let mut models = Vec::new();
    if keys.gemini.is_some() {
        models.push(
            std::env::var("GEMINI_MODEL")
                .ok()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| "gemini-3.5-flash-lite".to_string()),
        );
    }
    if keys.openai.is_some() {
        models.push(
            std::env::var("OPENAI_MODEL")
                .ok()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| "gpt-4o-mini".to_string()),
        );
    }
    models.push("local-tutor".to_string());
    models
}

fn clean_key(key: Option<&str>) -> Option<String> {
    key.map(str::trim).filter(|k| !k.is_empty()).map(str::to_string)
}

/// Pure dispatch decision: unit-testable without touching the environment.
pub fn plan_for(
    model: Option<&str>,
    byok_key: Option<&str>,
    keys: &ProviderKeys,
) -> Result<LlmPlan, String> {
    let name = model.map(str::trim).unwrap_or_default().to_lowercase();
    let byok = clean_key(byok_key);

    if name.is_empty() {
        return Ok(LlmPlan { provider: LlmProvider::BootDefault, model: String::new(), key: None });
    }
    if name == "local-tutor" {
        return Ok(LlmPlan { provider: LlmProvider::LocalTutor, model: name, key: None });
    }
    if name.starts_with("gemini") {
        let key = byok.or(keys.gemini.clone()).ok_or_else(|| {
            format!(
                "no API key for model '{name}': set GEMINI_API_KEY or provide apiKey with the request"
            )
        })?;
        return Ok(LlmPlan { provider: LlmProvider::Gemini, model: name, key: Some(key) });
    }
    if name.starts_with("gpt") || name.starts_with("o1") || name.starts_with("o3") {
        let key = byok.or(keys.openai.clone()).ok_or_else(|| {
            format!(
                "no API key for model '{name}': set OPENAI_API_KEY or provide apiKey with the request"
            )
        })?;
        return Ok(LlmPlan { provider: LlmProvider::OpenAI, model: name, key: Some(key) });
    }
    Ok(LlmPlan { provider: LlmProvider::BootDefault, model: name, key: None })
}

/// Build the client for a plan. Provider defaults everywhere; `BootDefault`
/// reuses the boot-configured client (which honors test injection and boot
/// env). The generic `OpenAiClient::with_base_url` remains available for a
/// future local/mobile relay; no provider routes to it today.
pub fn build_client(
    plan: &LlmPlan,
    boot_default: Arc<dyn llm::LlmClient>,
) -> Arc<dyn llm::LlmClient> {
    match &plan.provider {
        LlmProvider::Gemini => {
            Arc::new(llm::GeminiOpenAiClient::new(plan.key.clone().unwrap_or_default()))
        }
        LlmProvider::OpenAI => {
            Arc::new(llm::OpenAiClient::new(plan.key.clone().unwrap_or_default()))
        }
        LlmProvider::LocalTutor => Arc::new(llm::FakeLlmClient::new("local-tutor")),
        LlmProvider::BootDefault => boot_default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(gemini: bool, openai: bool) -> ProviderKeys {
        ProviderKeys {
            gemini: gemini.then(|| "server-gemini-key".to_string()),
            openai: openai.then(|| "server-openai-key".to_string()),
        }
    }

    #[test]
    fn gemini_prefers_byok_then_server_key_then_errors() {
        let plan =
            plan_for(Some("gemini-2.0-flash"), Some("  user-key "), &keys(true, false)).unwrap();
        assert_eq!(plan.provider, LlmProvider::Gemini);
        assert_eq!(plan.key.as_deref(), Some("user-key"));

        let plan = plan_for(Some("gemini-2.0-flash"), None, &keys(true, false)).unwrap();
        assert_eq!(plan.key.as_deref(), Some("server-gemini-key"));

        let err = plan_for(Some("gemini-2.0-flash"), None, &keys(false, false)).unwrap_err();
        assert!(err.contains("GEMINI_API_KEY"), "got: {err}");
    }

    #[test]
    fn openai_family_and_case_insensitive_match() {
        for model in ["gpt-4o-mini", "GPT-4o", "o1-mini", "o3"] {
            let plan = plan_for(Some(model), None, &keys(false, true)).unwrap();
            assert_eq!(plan.provider, LlmProvider::OpenAI, "for {model}");
        }
        assert!(plan_for(Some("gpt-4o"), None, &keys(false, false)).is_err());
    }

    #[test]
    fn ollama_names_fall_through_to_boot_default() {
        // Removed-for-now (mobile-first): no Ollama provider exists, so
        // these names resolve to the boot default instead of a local client.
        let plan = plan_for(Some("ollama-local"), None, &keys(false, false)).unwrap();
        assert_eq!(plan.provider, LlmProvider::BootDefault);
        assert_eq!(plan.key, None);

        let plan = plan_for(Some("  LOCAL-TUTOR "), None, &keys(true, true)).unwrap();
        assert_eq!(plan.provider, LlmProvider::LocalTutor);
    }

    #[test]
    fn absent_or_unknown_models_fall_back_to_boot_default() {
        for model in [None, Some(""), Some("   "), Some("mystery-9000")] {
            let plan = plan_for(model, None, &keys(true, true)).unwrap();
            assert_eq!(plan.provider, LlmProvider::BootDefault, "for {model:?}");
        }
    }
}
