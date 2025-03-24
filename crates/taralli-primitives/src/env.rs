use std::env;

/// configuration for clients/server to use api key auth or not
#[derive(Debug, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    #[must_use]
    pub fn from_env_var() -> Self {
        match env::var("ENV") {
            Ok(val) => match val.to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                "development" | "dev" => Environment::Development,
                _ => Environment::Development,
            },
            Err(_) => Environment::Development,
        }
    }

    /// Converts the `Environment` enum to a string for debugging or display purposes.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
        }
    }
}
