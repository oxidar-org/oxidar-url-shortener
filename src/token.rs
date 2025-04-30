use color_eyre::eyre::{self, eyre, Result};
use rand::Rng;
use std::fmt::{self, Display};

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

impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_length() {
        assert_eq!(Token::TOKEN_LENGTH, 6);
    }

    #[test]
    fn test_token_generation() {
        let token = Token::default();
        assert_eq!(token.as_str().len(), Token::TOKEN_LENGTH);
    }

    #[test]
    fn test_token_generation_is_random() {
        let token1 = Token::default();
        let token2 = Token::default();
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_token_generation_is_alphanumeric() {
        let token = Token::default();
        assert!(token.as_str().chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_try_from_fails_for_longer_strings() {
        let result = Token::try_from("1234567");
        assert!(result.is_err());
    }
}
