use crate::token::Token;
use color_eyre::eyre::{eyre, Result};
use std::collections::HashMap;
use url::Url;

#[derive(Default)]
pub struct Store {
    items: HashMap<Token, Url>,
}

pub trait StoreAccess {
    fn register_url(&mut self, url: Url) -> Result<Token>;
    fn resolve_token(&self, token: &str) -> Result<Url>;
}

impl StoreAccess for Store {
    fn register_url(&mut self, url: Url) -> Result<Token> {
        let token = Token::default();
        self.items.insert(token.clone(), url);
        Ok(token)
    }

    fn resolve_token(&self, token: &str) -> Result<Url> {
        let token = Token::try_from(token)?;
        self.items
            .get(&token)
            .cloned()
            .ok_or_else(|| eyre!("Token not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_url() -> Result<()> {
        let mut store = Store::default();
        let url = Url::parse("https://example.com")?;
        let token = store.register_url(url.clone())?;

        assert_eq!(store.items.len(), 1);
        assert_eq!(store.items.get(&token), Some(&url));
        Ok(())
    }

    #[test]
    fn test_resolve_token() -> Result<()> {
        let mut store = Store::default();
        let url = Url::parse("https://example.com")?;
        let token = store.register_url(url.clone())?;

        let resolved = store.resolve_token(token.as_str())?;
        assert_eq!(resolved, url);
        Ok(())
    }

    #[test]
    fn test_resolve_nonexistent_token() {
        let store = Store::default();
        let result = store.resolve_token("123456");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_invalid_token() {
        let store = Store::default();
        let result = store.resolve_token("too_long");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_urls() -> Result<()> {
        let mut store = Store::default();
        let url1 = Url::parse("https://example1.com")?;
        let url2 = Url::parse("https://example2.com")?;

        let token1 = store.register_url(url1.clone())?;
        let token2 = store.register_url(url2.clone())?;

        assert_ne!(token1, token2);
        assert_eq!(store.resolve_token(token1.as_str())?, url1);
        assert_eq!(store.resolve_token(token2.as_str())?, url2);
        Ok(())
    }
}
