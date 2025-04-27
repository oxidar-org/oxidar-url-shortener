use crate::store::{Store, StoreAccess};
use axum::{
    extract::{Path, Request, State},
    http,
    response::Redirect,
    routing::{get, post},
    Router,
};
use color_eyre::eyre::{eyre, Result};
use std::sync::{Arc, Mutex};
use url::Url;

#[derive(Default)]
pub struct AppState {
    pub store: Store,
}

pub fn extract_base_url(req: &Request) -> Result<Url> {
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

pub async fn extract_body_url(req: Request) -> Result<Url> {
    let body = axum::body::to_bytes(req.into_body(), usize::MAX).await?;
    let str = std::str::from_utf8(&body)?;
    Url::parse(str).map_err(|e| eyre!("Failed to parse URL: {}", e))
}

pub async fn resolve_url(
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

pub async fn register_url(
    State(state): State<Arc<Mutex<AppState>>>,
    req: Request,
) -> Result<String, http::StatusCode> {
    let base_url = extract_base_url(&req).map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let target_url = extract_body_url(req)
        .await
        .map_err(|_| http::StatusCode::BAD_REQUEST)?;

    let token = {
        let mut state = state.lock().map_err(|_| http::StatusCode::LOCKED)?;
        state
            .store
            .register_url(target_url)
            .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let resolved = base_url
        .join(token.as_str())
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(resolved.to_string())
}

pub fn create_router() -> Router {
    let state = Arc::new(Mutex::new(AppState::default()));
    Router::new()
        .route("/{token}", get(resolve_url))
        .route("/", post(register_url))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::str::FromStr;

    #[test]
    fn test_extract_base_url() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-host", "example.com".parse().unwrap());

        let mut req = Request::builder()
            .uri("http://example.com")
            .body(axum::body::Body::empty())
            .unwrap();
        req.headers_mut().extend(headers);

        let result = extract_base_url(&req).unwrap();
        assert_eq!(result.scheme(), "https");
        assert_eq!(result.host_str().unwrap(), "example.com");
    }

    #[test]
    fn test_extract_base_url_fallback() {
        let req = Request::builder()
            .uri("http://localhost:3000")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_base_url(&req).unwrap();
        assert_eq!(result.scheme(), "http");
        assert_eq!(result.host_str().unwrap(), "localhost");
    }

    #[tokio::test]
    async fn test_extract_body_url() {
        let url = "https://example.com";
        let req = Request::builder()
            .uri("http://localhost:3000")
            .body(axum::body::Body::from(url))
            .unwrap();

        let result = extract_body_url(req).await.unwrap();
        assert_eq!(result.to_string(), "https://example.com/");
    }

    #[tokio::test]
    async fn test_resolve_url() {
        let state = Arc::new(Mutex::new(AppState::default()));
        let token = {
            let mut state_guard = state.lock().unwrap();
            state_guard
                .store
                .register_url(Url::from_str("https://example.com").unwrap())
                .unwrap()
        };

        let result = resolve_url(State(state), Path(token.as_str().to_string())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_url() {
        let state = Arc::new(Mutex::new(AppState::default()));
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-host", "example.com".parse().unwrap());

        let mut req = Request::builder()
            .uri("http://example.com")
            .body(axum::body::Body::from("https://target.com"))
            .unwrap();
        req.headers_mut().extend(headers);

        let result = register_url(State(state), req).await;
        assert!(result.is_ok());
        let short_url = result.unwrap();
        assert!(short_url.starts_with("https://example.com/"));
    }
}
