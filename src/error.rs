use crate::prelude::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("Failed to convert time to UTC format")]
    ParseTime,
    #[error(transparent)]
    ParseError(#[from] chrono::format::ParseError),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    WatchId(#[from] WatchIdError),
}

pub type Result<T> = std::result::Result<T, WatchError>;
pub type AsyncResult<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;
