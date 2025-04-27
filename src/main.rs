mod store;
mod token;

use axum::{
    extract::{Path, Request, State},
    http,
    response::Redirect,
    routing::{get, post},
    Router,
};
use color_eyre::eyre::{eyre, Result};
use std::sync::{Arc, Mutex};
use store::{Store, StoreAccess};
use url::Url;

#[derive(Default)]
struct AppState {
    pub store: Store,
}

async fn resolve_url(
    State(state): State<Arc<Mutex<AppState>>>,
    Path(token): Path<String>,
) -> Result<Redirect, http::StatusCode> {
    let state = state.lock().map_err(|_| http::StatusCode::LOCKED)?;
    let url = state
        .store
        .resolve_token(&token)
        .map_err(|_| http::StatusCode::NOT_FOUND)
        .map(|u| u.to_string())?;

    Ok(Redirect::to(&url))
}

fn extract_base_url(req: &Request) -> Result<Url> {
    let headers = req.headers();

    // Check for forwarded protocol (https/http)
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("http");

    // Check for forwarded host
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    Url::parse(&format!("{}://{}", proto, host))
        .map_err(|e| eyre!("Failed to parse base URL: {}", e))
}

async fn extract_body_url(req: Request) -> Result<Url> {
    let body = axum::body::to_bytes(req.into_body(), usize::MAX).await?;
    let str = std::str::from_utf8(&body)?;
    Url::parse(str).map_err(|e| eyre!("Failed to parse URL: {}", e))
}

async fn register_url(
    State(state): State<Arc<Mutex<AppState>>>,
    req: Request,
) -> Result<String, http::StatusCode> {
    let base_url = extract_base_url(&req).map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let target_url = extract_body_url(req)
        .await
        .map_err(|_| http::StatusCode::BAD_REQUEST)?;

    let mut state = state.lock().map_err(|_| http::StatusCode::LOCKED)?;
    let token = state
        .store
        .register_url(target_url)
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    drop(state);

    let resolved = base_url
        .join(token.as_str())
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(resolved.to_string())
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    color_eyre::install().unwrap();

    let state = Arc::new(Mutex::new(AppState::default()));
    let router = Router::new()
        .route("/{token}", get(resolve_url))
        .route("/", post(register_url))
        .with_state(state);

    Ok(router.into())
}
