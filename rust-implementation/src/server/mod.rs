// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HTTP SERVER - OpenAI-Compatible API com SSE Streaming
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//!
//! Servidor HTTP compatível com a API OpenAI Chat Completions.
//!
//! ## Endpoints
//!
//! - `GET /health` - Health check
//! - `GET /v1/models` - Lista modelos disponíveis
//! - `GET /v1/models/{model}` - Detalhes de um modelo
//! - `POST /v1/chat/completions` - Pesquisa com SSE streaming ou JSON
//!
//! ## Uso
//!
//! ```bash
//! cargo run --features server -- --server --port=3000
//! cargo run --features server -- --server --port=3000 --secret=minha-chave
//! ```

#[allow(missing_docs)]
pub mod types;
#[allow(missing_docs)]
pub mod handlers;
#[allow(missing_docs)]
pub mod sse;
mod auth;

use std::net::SocketAddr;
use std::sync::Arc;

pub use types::*;

/// Estado compartilhado entre todos os handlers
pub struct AppState {
    /// Configuração do LLM
    pub llm_config: crate::config::LlmConfig,
    /// Configuração do runtime
    pub runtime_config: crate::config::RuntimeConfig,
    /// Configuração do agente
    pub agent_config: crate::config::AgentConfig,
    /// Chave da API OpenAI
    pub openai_key: String,
    /// Chave da API Jina
    pub jina_key: String,
    /// Token de autenticação opcional (Bearer)
    pub secret: Option<String>,
}

/// Inicia o servidor HTTP no endereço especificado.
///
/// Entry point chamado de main.rs quando `--server` é passado.
pub async fn start_server(addr: SocketAddr, state: Arc<AppState>) -> anyhow::Result<()> {
    use axum::{middleware, routing::{get, post}, Router};
    use tower_http::cors::CorsLayer;

    let routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/models", get(handlers::list_models))
        .route("/v1/models/{model}", get(handlers::get_model))
        .route("/v1/chat/completions", post(handlers::chat_completions));

    // Auth middleware condicional
    let routes = if state.secret.is_some() {
        routes.layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
    } else {
        routes
    };

    // CORS + state → Router<()> (pronto para serve)
    let app = routes
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("DeepResearch server listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
