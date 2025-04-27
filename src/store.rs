use color_eyre::eyre::{eyre, Result};
use rand::Rng;
use std::collections::HashMap;
use url::Url;

pub struct Store {
    items: HashMap<String, Url>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    fn generate_token() -> String {
        rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(6)
            .map(char::from)
            .collect()
    }
}

pub trait StoreAccess {
    fn register_url(&mut self, url: Url) -> Result<String>;
    fn resolve_token(&self, token: &str) -> Result<Url>;
}

impl StoreAccess for Store {
    fn register_url(&mut self, url: Url) -> Result<String> {
        let token = Self::generate_token();
        self.items.insert(token.clone(), url);
        Ok(token)
    }

    fn resolve_token(&self, token: &str) -> Result<Url> {
        self.items
            .get(token)
            .cloned()
            .ok_or_else(|| eyre!("Token not found"))
    }
}
