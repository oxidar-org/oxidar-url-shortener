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
