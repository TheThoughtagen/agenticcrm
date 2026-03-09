use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpsError {
    #[error("no contact matching '{0}'")]
    NotFound(String),

    #[error("multiple contacts match '{query}': {matches}")]
    AmbiguousMatch { query: String, matches: String },

    #[error("validation failed: {0}")]
    ValidationFailed(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Internal(String),
}
