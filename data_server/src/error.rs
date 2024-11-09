use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataServerError {
    #[error("Generic error")]
    Generic,
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
}

pub(crate) type Result<T> = std::result::Result<T, DataServerError>;
