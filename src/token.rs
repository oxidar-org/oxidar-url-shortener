use color_eyre::eyre::{self, eyre, Result};
use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token(String);

impl Default for Token {
    fn default() -> Self {
        let str = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(Self::TOKEN_LENGTH)
            .map(char::from)
            .collect();
        Self(str)
    }
}

impl Token {
    const TOKEN_LENGTH: usize = 6;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Token {
    type Error = eyre::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != Self::TOKEN_LENGTH {
            return Err(eyre!(
                "Token must be {} characters long",
                Self::TOKEN_LENGTH
            ));
        }
        Ok(Self(value.to_string()))
    }
}
