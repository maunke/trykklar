//! Error and Result types

use pdf::error::FieldError;
use std::sync::Arc;

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(Arc::new(e))
    }
}

impl From<&pdf::Error> for Error {
    fn from(e: &pdf::Error) -> Self {
        Error::Pdf(Arc::new(e.clone()))
    }
}

impl From<pdf::Error> for Error {
    fn from(e: pdf::Error) -> Self {
        Error::Pdf(Arc::new(e))
    }
}

impl From<FieldError> for Error {
    fn from(e: FieldError) -> Self {
        Error::Pdf(Arc::new(e.into()))
    }
}

/// Error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Trykklar PDF error
    #[error(transparent)]
    Pdf(#[from] Arc<trykklar_pdf::Error>),
    /// IO error
    #[error("IO error: {0}")]
    Io(#[source] Arc<std::io::Error>),
    /// Value is not finite.
    #[error("non finite value")]
    NonFinite,
}

/// Result
pub type Result<T> = std::result::Result<T, Error>;
