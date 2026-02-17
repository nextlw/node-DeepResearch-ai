// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SSE STREAMING - Bridge AgentProgress → Server-Sent Events
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use futures::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::agent::{AgentProgress, DeepResearchAgent};
use super::types::*;

/// Payload interno enviado pelo broadcast channel
#[derive(Debug, Clone)]
pub enum SsePayload {
    /// Evento de progresso do agente
    Progress(AgentProgress),
    /// Agente finalizou (resultado final)
    Completed(CompletedPayload),
}

/// Dados finais do agente (subset clonável de ResearchResult)
#[derive(Debug, Clone)]
pub struct CompletedPayload {
    pub success: bool,
    pub answer: Option<String>,
    pub references: Vec<crate::types::Reference>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub visited_urls: Vec<String>,
    pub error: Option<String>,
}

/// Cria e retorna uma resposta SSE para streaming do agente.
///
/// 1. Cria um broadcast channel
/// 2. Wires o ProgressCallback para enviar AgentProgress no channel
/// 3. Spawna o agente em uma task tokio
/// 4. Retorna Sse<Stream> que consome o channel e emite chunks JSON
pub async fn handle_streaming(
    llm_client: Arc<dyn crate::llm::LlmClient>,
    search_client: Arc<dyn crate::search::SearchClient>,
    question: String,
    token_budget: u64,
    request_id: String,
    created: i64,
    model: String,
) -> Response {
    let (tx, _) = broadcast::channel::<SsePayload>(512);
    let tx_callback = tx.clone();
    let tx_completion = tx.clone();

    // ProgressCallback síncrono → broadcast::send (non-blocking)
    let progress_callback: crate::agent::ProgressCallback =
        Arc::new(move |event: AgentProgress| {
            let _ = tx_callback.send(SsePayload::Progress(event));
        });

    // Spawnar agente em background task
    tokio::spawn(async move {
        let agent = DeepResearchAgent::new(llm_client, search_client, Some(token_budget))
            .with_progress_callback(progress_callback);

        let result = agent.run(question).await;

        let _ = tx_completion.send(SsePayload::Completed(CompletedPayload {
            success: result.success,
            answer: result.answer,
            references: result.references,
            prompt_tokens: result.token_usage.prompt_tokens,
            completion_tokens: result.token_usage.completion_tokens,
            total_tokens: result.token_usage.total_tokens,
            visited_urls: result.visited_urls,
            error: result.error,
        }));
    });

    // Criar stream SSE a partir do broadcast receiver
    let rx = tx.subscribe();
    let stream = build_sse_stream(rx, request_id, created, model);

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// Constrói o stream SSE a partir do broadcast receiver.
///
/// Emite:
/// 1. Chunk inicial com `<think>` e `role: "assistant"`
/// 2. Chunks de progresso (think, url, query)
/// 3. Chunk de `</think>` com finish_reason: "thinking_end"
/// 4. Chunk final com resposta, annotations, usage, visitedURLs
fn build_sse_stream(
    rx: broadcast::Receiver<SsePayload>,
    request_id: String,
    created: i64,
    model: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let rid = request_id.clone();
    let mdl = model.clone();

    // 1. Chunk inicial: <think>
    let initial_chunk = make_chunk(
        &rid,
        created,
        &mdl,
        ChunkDelta {
            role: Some("assistant".into()),
            content: Some("<think>".into()),
            delta_type: Some("think".into()),
            url: None,
            query: None,
            annotations: None,
        },
        None,
    );
    let initial_json = serde_json::to_string(&initial_chunk).unwrap_or_default();
    let initial_event =
        futures::stream::once(async move { Ok::<_, Infallible>(Event::default().data(initial_json)) });

    // 2. Stream de eventos do broadcast → flat_map para SSE Events
    let broadcast_stream = BroadcastStream::new(rx);

    let event_stream = broadcast_stream
        .filter_map(move |msg| {
            let rid = rid.clone();
            let mdl = mdl.clone();
            async move {
                match msg {
                    Ok(SsePayload::Progress(progress)) => {
                        let jsons = progress_to_events(&progress, &rid, created, &mdl);
                        if jsons.is_empty() {
                            None
                        } else {
                            Some(jsons)
                        }
                    }
                    Ok(SsePayload::Completed(result)) => {
                        Some(completion_to_events(&result, &rid, created, &mdl))
                    }
                    Err(_) => None,
                }
            }
        })
        .flat_map(|jsons| {
            stream::iter(
                jsons
                    .into_iter()
                    .map(|json| Ok::<_, Infallible>(Event::default().data(json))),
            )
        });

    initial_event.chain(event_stream)
}

/// Converte AgentProgress em 0+ chunks JSON SSE
fn progress_to_events(
    progress: &AgentProgress,
    request_id: &str,
    created: i64,
    model: &str,
) -> Vec<String> {
    let mut events = Vec::new();

    match progress {
        AgentProgress::Think(content) => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: Some(format!("{} ", content)),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::VisitedUrl(url) => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: None,
                    delta_type: Some("think".into()),
                    url: Some(url.clone()),
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::PersonaQuery { expanded, .. } => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: None,
                    delta_type: Some("think".into()),
                    url: None,
                    query: Some(expanded.clone()),
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::Action(action) => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: Some(format!("{} ", action)),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::Info(msg) | AgentProgress::Success(msg) => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: Some(format!("{} ", msg)),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::Warning(msg) => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: Some(format!("[warning] {} ", msg)),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::Error(msg) => {
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: Some(format!("[error] {} ", msg)),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        AgentProgress::ValidationStep {
            eval_type,
            passed,
            confidence,
            ..
        } => {
            let status = if *passed { "PASS" } else { "FAIL" };
            let chunk = make_chunk(
                request_id,
                created,
                model,
                ChunkDelta {
                    role: None,
                    content: Some(format!(
                        "[{}] {} (confidence: {:.0}%) ",
                        status, eval_type, confidence * 100.0
                    )),
                    delta_type: Some("think".into()),
                    url: None,
                    query: None,
                    annotations: None,
                },
                None,
            );
            push_json(&mut events, &chunk);
        }
        // Outros eventos são silenciados no SSE
        _ => {}
    }

    events
}

/// Converte resultado final em chunks SSE (thinking_end + stop)
fn completion_to_events(
    result: &CompletedPayload,
    request_id: &str,
    created: i64,
    model: &str,
) -> Vec<String> {
    let mut events = Vec::new();

    // 1. Fechar tag </think>
    let close_think = make_chunk(
        request_id,
        created,
        model,
        ChunkDelta {
            role: None,
            content: Some("</think>\n\n".into()),
            delta_type: Some("think".into()),
            url: None,
            query: None,
            annotations: None,
        },
        Some("thinking_end"),
    );
    push_json(&mut events, &close_think);

    // 2. Chunk final com resposta
    let (content, content_type, finish_reason) = if result.success {
        (
            result.answer.clone().unwrap_or_default(),
            "text",
            "stop",
        )
    } else {
        let err = result.error.clone().unwrap_or_else(|| "Unknown error".into());
        (err, "error", "error")
    };

    let annotations = build_annotations(&result.references);
    let usage = UsageInfo {
        prompt_tokens: result.prompt_tokens,
        completion_tokens: result.completion_tokens,
        total_tokens: result.total_tokens,
    };

    let final_chunk = ChatCompletionChunk {
        id: request_id.into(),
        object: "chat.completion.chunk".into(),
        created,
        model: model.into(),
        system_fingerprint: format!("fp_{}", request_id),
        choices: vec![ChunkChoice {
            index: 0,
            delta: ChunkDelta {
                role: None,
                content: Some(content),
                delta_type: Some(content_type.into()),
                url: None,
                query: None,
                annotations,
            },
            logprobs: None,
            finish_reason: Some(finish_reason.into()),
        }],
        usage: Some(usage),
        visited_urls: Some(result.visited_urls.clone()),
        read_urls: None,
        num_urls: Some(result.visited_urls.len()),
    };
    push_json(&mut events, &final_chunk);

    events
}

// ─────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────

fn make_chunk(
    request_id: &str,
    created: i64,
    model: &str,
    delta: ChunkDelta,
    finish_reason: Option<&str>,
) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: request_id.into(),
        object: "chat.completion.chunk".into(),
        created,
        model: model.into(),
        system_fingerprint: format!("fp_{}", request_id),
        choices: vec![ChunkChoice {
            index: 0,
            delta,
            logprobs: None,
            finish_reason: finish_reason.map(|s| s.into()),
        }],
        usage: None,
        visited_urls: None,
        read_urls: None,
        num_urls: None,
    }
}

fn push_json(events: &mut Vec<String>, chunk: &ChatCompletionChunk) {
    if let Ok(json) = serde_json::to_string(chunk) {
        events.push(json);
    }
}
