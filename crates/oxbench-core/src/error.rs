use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("{0}")]
    InvalidInput(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
