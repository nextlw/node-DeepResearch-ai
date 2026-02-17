// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AUTENTICAÇÃO - Bearer Token Middleware
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;

use super::AppState;
use super::types::{ApiError, ApiErrorDetail};

/// Middleware de autenticação Bearer token.
///
/// Ativado apenas quando `--secret=TOKEN` é passado na inicialização.
/// Endpoints públicos (/health, GET /v1/models) são excluídos.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let secret = match &state.secret {
        Some(s) => s,
        None => return next.run(request).await,
    };

    // Endpoints públicos - sem auth
    let path = request.uri().path();
    if path == "/health" {
        return next.run(request).await;
    }
    if path.starts_with("/v1/models") && request.method() == "GET" {
        return next.run(request).await;
    }

    // Verificar Bearer token
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if token == secret {
                next.run(request).await
            } else {
                unauthorized_response()
            }
        }
        _ => unauthorized_response(),
    }
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiError {
            error: ApiErrorDetail {
                message: "Unauthorized. Please provide a valid API key.".into(),
                error_type: "invalid_request_error".into(),
                param: None,
                code: Some("unauthorized".into()),
            },
        }),
    )
        .into_response()
}
