// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ENDPOINT HANDLERS - 4 endpoints compatíveis com OpenAI API
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::sse;
use super::types::*;
use super::AppState;
use crate::agent::DeepResearchAgent;
use crate::llm::OpenAiClient;
use crate::search::JinaClient;

// ── GET /health ─────────────────────────────────

/// Health check endpoint
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

// ── GET /v1/models ──────────────────────────────

/// Lista modelos disponíveis
pub async fn list_models() -> Json<ModelList> {
    Json(ModelList {
        object: "list".into(),
        data: available_models(),
    })
}

// ── GET /v1/models/{model} ──────────────────────

/// Retorna detalhes de um modelo específico
pub async fn get_model(Path(model_id): Path<String>) -> Response {
    match available_models().into_iter().find(|m| m.id == model_id) {
        Some(model) => Json(model).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: ApiErrorDetail {
                    message: format!("Model '{}' not found", model_id),
                    error_type: "invalid_request_error".into(),
                    param: None,
                    code: Some("model_not_found".into()),
                },
            }),
        )
            .into_response(),
    }
}

// ── POST /v1/chat/completions ───────────────────

/// Endpoint principal: pesquisa com streaming (SSE) ou resposta JSON
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatCompletionRequest>,
) -> Response {
    // Validar messages
    if body.messages.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "The \"messages\" parameter is required and must not be empty.",
        );
    }

    let last_msg = &body.messages[body.messages.len() - 1];
    if last_msg.role != "user" {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Last message must be from user role.",
        );
    }

    // Extrair pergunta
    let question = extract_question(&last_msg.content);
    if question.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Could not extract question from last message content.",
        );
    }

    // Calcular token budget
    let token_budget = resolve_token_budget(
        body.reasoning_effort.as_deref(),
        body.max_completion_tokens,
        body.budget_tokens,
    );

    let request_id = format!("req_{}", now_millis());
    let created = now_secs();
    let model = body.model.clone();

    // Criar clientes (mesmo padrão de spawn_research_task no main.rs)
    let llm_client: Arc<dyn crate::llm::LlmClient> =
        Arc::new(OpenAiClient::from_config(state.openai_key.clone(), &state.llm_config));
    let search_client: Arc<dyn crate::search::SearchClient> =
        Arc::new(JinaClient::with_preference(
            state.jina_key.clone(),
            state.runtime_config.webreader,
        ));

    if body.stream {
        // SSE streaming
        log::info!("[SSE] Starting streaming research: {}", question);
        sse::handle_streaming(
            llm_client,
            search_client,
            question,
            token_budget,
            request_id,
            created,
            model,
        )
        .await
    } else {
        // Resposta JSON completa
        log::info!("[JSON] Starting research: {}", question);
        handle_non_streaming(
            llm_client,
            search_client,
            question,
            token_budget,
            request_id,
            created,
            model,
        )
        .await
    }
}

// ── Non-streaming handler ───────────────────────

async fn handle_non_streaming(
    llm_client: Arc<dyn crate::llm::LlmClient>,
    search_client: Arc<dyn crate::search::SearchClient>,
    question: String,
    token_budget: u64,
    request_id: String,
    created: i64,
    model: String,
) -> Response {
    let agent = DeepResearchAgent::new(llm_client, search_client, Some(token_budget));

    match tokio::spawn(async move { agent.run(question).await }).await {
        Ok(result) => {
            let (content, content_type, finish_reason) = if result.success {
                let answer = result.answer.unwrap_or_default();
                (answer, "text", "stop")
            } else {
                let err = result.error.unwrap_or_else(|| "Unknown error".into());
                (format!("Error: {}", err), "error", "error")
            };

            let annotations = build_annotations(&result.references);
            let visited = result.visited_urls.clone();

            let response = ChatCompletionResponse {
                id: request_id.clone(),
                object: "chat.completion".into(),
                created,
                model,
                system_fingerprint: format!("fp_{}", request_id),
                choices: vec![ChatCompletionChoice {
                    index: 0,
                    message: ChatCompletionMessage {
                        role: "assistant".into(),
                        content,
                        content_type: content_type.into(),
                        annotations,
                    },
                    logprobs: None,
                    finish_reason: finish_reason.into(),
                }],
                usage: UsageInfo {
                    prompt_tokens: result.token_usage.prompt_tokens,
                    completion_tokens: result.token_usage.completion_tokens,
                    total_tokens: result.token_usage.total_tokens,
                },
                visited_urls: Some(visited.clone()),
                read_urls: None,
                num_urls: Some(visited.len()),
            };

            Json(response).into_response()
        }
        Err(e) => {
            log::error!("[chat/completions] Agent task panicked: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Internal error: {}", e),
            )
        }
    }
}

// ── Helpers ─────────────────────────────────────

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            error: ApiErrorDetail {
                message: message.into(),
                error_type: "invalid_request_error".into(),
                param: None,
                code: None,
            },
        }),
    )
        .into_response()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
