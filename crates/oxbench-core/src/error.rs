use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("{0}")]
    InvalidInput(String),

    /// `line` and `column` are 1-based, as editors show them.
    #[error("Line {line}, column {column}: {message}")]
    InvalidJson {
        message: String,
        line: usize,
        column: usize,
    },
}

pub type CoreResult<T> = Result<T, CoreError>;
