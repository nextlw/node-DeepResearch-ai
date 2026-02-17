// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SCHEMAS API - Compatível com OpenAI Chat Completions
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────
// Model
// ─────────────────────────────────────────────────

/// Modelo disponível na API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    /// Sempre "model"
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

/// Lista de modelos
#[derive(Debug, Serialize)]
pub struct ModelList {
    /// Sempre "list"
    pub object: String,
    pub data: Vec<Model>,
}

// ─────────────────────────────────────────────────
// Chat Completion Request
// ─────────────────────────────────────────────────

/// Mensagem do chat (role + content)
#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    /// String simples ou array de content parts (multimodal)
    pub content: serde_json::Value,
}

/// Request para POST /v1/chat/completions
#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    /// "low" | "medium" | "high"
    pub reasoning_effort: Option<String>,
    pub max_completion_tokens: Option<u64>,
    pub budget_tokens: Option<u64>,
    pub max_attempts: Option<u32>,
    pub no_direct_answer: Option<bool>,
    pub max_returned_urls: Option<usize>,
    pub boost_hostnames: Option<Vec<String>>,
    pub bad_hostnames: Option<Vec<String>>,
    pub only_hostnames: Option<Vec<String>>,
    pub max_annotations: Option<usize>,
    pub min_annotation_relevance: Option<f32>,
    pub with_images: Option<bool>,
    pub language_code: Option<String>,
    pub search_language_code: Option<String>,
    pub search_provider: Option<String>,
    pub team_size: Option<usize>,
}

// ─────────────────────────────────────────────────
// Annotations (URL Citations)
// ─────────────────────────────────────────────────

/// Citação de URL em uma resposta
#[derive(Debug, Clone, Serialize)]
pub struct URLAnnotation {
    #[serde(rename = "type")]
    pub annotation_type: String,
    pub url_citation: URLCitation,
}

/// Detalhes da citação
#[derive(Debug, Clone, Serialize)]
pub struct URLCitation {
    pub title: String,
    #[serde(rename = "exactQuote")]
    pub exact_quote: String,
    pub url: String,
    #[serde(rename = "dateTime", skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>,
}

// ─────────────────────────────────────────────────
// Chat Completion Response (non-streaming)
// ─────────────────────────────────────────────────

/// Mensagem na resposta (role + content + type)
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    pub content: String,
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<URLAnnotation>>,
}

/// Choice na resposta completa
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: String,
}

/// Estatísticas de uso de tokens
#[derive(Debug, Clone, Serialize)]
pub struct UsageInfo {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Resposta completa (non-streaming)
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    /// Sempre "chat.completion"
    pub object: String,
    pub created: i64,
    pub model: String,
    pub system_fingerprint: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: UsageInfo,
    #[serde(rename = "visitedURLs", skip_serializing_if = "Option::is_none")]
    pub visited_urls: Option<Vec<String>>,
    #[serde(rename = "readURLs", skip_serializing_if = "Option::is_none")]
    pub read_urls: Option<Vec<String>>,
    #[serde(rename = "numURLs", skip_serializing_if = "Option::is_none")]
    pub num_urls: Option<usize>,
}

// ─────────────────────────────────────────────────
// Chat Completion Chunk (SSE streaming)
// ─────────────────────────────────────────────────

/// Delta incremental no chunk SSE
#[derive(Debug, Clone, Serialize)]
pub struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub delta_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<URLAnnotation>>,
}

/// Choice no chunk SSE
#[derive(Debug, Clone, Serialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: Option<String>,
}

/// Chunk SSE para streaming (formato `data: {JSON}\n\n`)
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    /// Sempre "chat.completion.chunk"
    pub object: String,
    pub created: i64,
    pub model: String,
    pub system_fingerprint: String,
    pub choices: Vec<ChunkChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,
    #[serde(rename = "visitedURLs", skip_serializing_if = "Option::is_none")]
    pub visited_urls: Option<Vec<String>>,
    #[serde(rename = "readURLs", skip_serializing_if = "Option::is_none")]
    pub read_urls: Option<Vec<String>>,
    #[serde(rename = "numURLs", skip_serializing_if = "Option::is_none")]
    pub num_urls: Option<usize>,
}

// ─────────────────────────────────────────────────
// Error Response
// ─────────────────────────────────────────────────

/// Resposta de erro da API
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ApiErrorDetail,
}

/// Detalhes do erro
#[derive(Debug, Serialize)]
pub struct ApiErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

// ─────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────

/// Retorna a lista de modelos disponíveis
pub fn available_models() -> Vec<Model> {
    vec![
        Model {
            id: "jina-deepsearch-v1".into(),
            object: "model".into(),
            created: 1686935002,
            owned_by: "jina-ai".into(),
        },
        Model {
            id: "jina-deepsearch-v2".into(),
            object: "model".into(),
            created: 1717987200,
            owned_by: "jina-ai".into(),
        },
    ]
}

/// Extrai o texto da pergunta a partir do content de uma mensagem
pub fn extract_question(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if item.get("type")?.as_str()? == "text" {
                    item.get("text")?.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Calcula o budget de tokens a partir dos parâmetros do request
pub fn resolve_token_budget(
    reasoning_effort: Option<&str>,
    max_completion_tokens: Option<u64>,
    budget_tokens: Option<u64>,
) -> u64 {
    if let Some(budget) = budget_tokens {
        return budget;
    }
    if let Some(max) = max_completion_tokens {
        return max;
    }
    match reasoning_effort {
        Some("low") => 100_000,
        Some("high") => 1_000_000,
        _ => 500_000, // medium ou default
    }
}

/// Constrói annotations a partir das referências do agente
pub fn build_annotations(references: &[crate::types::Reference]) -> Option<Vec<URLAnnotation>> {
    let annots: Vec<URLAnnotation> = references
        .iter()
        .filter(|r| !r.url.is_empty() && !r.title.is_empty())
        .map(|r| URLAnnotation {
            annotation_type: "url_citation".into(),
            url_citation: URLCitation {
                title: r.title.clone(),
                exact_quote: r.exact_quote.clone().unwrap_or_default(),
                url: r.url.clone(),
                date_time: None,
            },
        })
        .collect();
    if annots.is_empty() {
        None
    } else {
        Some(annots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_question_string() {
        let content = serde_json::Value::String("What is Rust?".into());
        assert_eq!(extract_question(&content), "What is Rust?");
    }

    #[test]
    fn test_extract_question_array() {
        let content = serde_json::json!([
            {"type": "text", "text": "Hello"},
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,..."} },
            {"type": "text", "text": "World"}
        ]);
        assert_eq!(extract_question(&content), "Hello World");
    }

    #[test]
    fn test_resolve_token_budget() {
        assert_eq!(resolve_token_budget(None, None, Some(42)), 42);
        assert_eq!(resolve_token_budget(None, Some(99), None), 99);
        assert_eq!(resolve_token_budget(Some("low"), None, None), 100_000);
        assert_eq!(resolve_token_budget(Some("high"), None, None), 1_000_000);
        assert_eq!(resolve_token_budget(Some("medium"), None, None), 500_000);
        assert_eq!(resolve_token_budget(None, None, None), 500_000);
    }

    #[test]
    fn test_available_models() {
        let models = available_models();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "jina-deepsearch-v1");
        assert_eq!(models[1].id, "jina-deepsearch-v2");
    }

    #[test]
    fn test_chunk_serialization() {
        let chunk = ChatCompletionChunk {
            id: "req_123".into(),
            object: "chat.completion.chunk".into(),
            created: 1700000000,
            model: "jina-deepsearch-v1".into(),
            system_fingerprint: "fp_req_123".into(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta {
                    role: Some("assistant".into()),
                    content: Some("<think>".into()),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
            visited_urls: None,
            read_urls: None,
            num_urls: None,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("chat.completion.chunk"));
        assert!(json.contains("<think>"));
        // skip_serializing_if = None fields should not appear
        assert!(!json.contains("visitedURLs"));
    }
}
