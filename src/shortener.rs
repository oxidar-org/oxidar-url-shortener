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

pub fn create_router() -> Router {
    let state = Arc::new(Mutex::new(AppState::default()));
    Router::new()
        .route("/{token}", get(resolve_url))
        .route("/", post(register_url))
        .with_state(state)
}

struct AppState {
    pub store: Box<dyn StoreAccess>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: Box::new(Store::default()),
        }
    }
}

// Helpers
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

// Routes
async fn extract_body_url(req: Request) -> Result<Url> {
    let body = axum::body::to_bytes(req.into_body(), usize::MAX).await?;
    let str = std::str::from_utf8(&body)?;
    Url::parse(str).map_err(|e| eyre!("Failed to parse URL: {}", e))
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

async fn register_url(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Token;
    use axum::http::HeaderMap;
    use axum::response::IntoResponse;
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::Mutex;

    // Mock store implementation
    struct MockStore {
        urls: Mutex<HashMap<String, Url>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                urls: Mutex::new(HashMap::new()),
            }
        }

        fn with_url(self, token: &str, url: Url) -> Self {
            self.urls.lock().unwrap().insert(token.to_string(), url);
            self
        }
    }

    impl StoreAccess for MockStore {
        fn register_url(&mut self, url: Url) -> Result<Token> {
            let token = Token::default();
            self.urls
                .lock()
                .unwrap()
                .insert(token.as_str().to_string(), url);
            Ok(token)
        }

        fn resolve_token(&self, token: &str) -> Result<Url> {
            self.urls
                .lock()
                .unwrap()
                .get(token)
                .cloned()
                .ok_or_else(|| eyre!("Token not found"))
        }
    }

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

    #[tokio::test]
    async fn test_resolve_url_with_mock_store() {
        let mock_store =
            MockStore::new().with_url("abc123", Url::parse("https://example.com").unwrap());
        let state = Arc::new(Mutex::new(AppState {
            store: Box::new(mock_store),
        }));

        let result = resolve_url(State(state), Path("abc123".to_string())).await;
        assert!(result.is_ok());
        let redirect = result.unwrap();
        let response = redirect.into_response();
        let headers = response.headers();
        assert_eq!(headers.get("location").unwrap(), "https://example.com/");
    }

    #[tokio::test]
    async fn test_resolve_url_not_found() {
        let mock_store = MockStore::new();
        let state = Arc::new(Mutex::new(AppState {
            store: Box::new(mock_store),
        }));

        let result = resolve_url(State(state), Path("nonexistent".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_register_url_with_mock_store() {
        let mock_store = MockStore::new();
        let state = Arc::new(Mutex::new(AppState {
            store: Box::new(mock_store),
        }));

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

    #[tokio::test]
    async fn test_register_url_invalid_url() {
        let mock_store = MockStore::new();
        let state = Arc::new(Mutex::new(AppState {
            store: Box::new(mock_store),
        }));

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-host", "example.com".parse().unwrap());

        let mut req = Request::builder()
            .uri("http://example.com")
            .body(axum::body::Body::from("not-a-url"))
            .unwrap();
        req.headers_mut().extend(headers);

        let result = register_url(State(state), req).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), http::StatusCode::BAD_REQUEST);
    }
}
