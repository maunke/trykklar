//! Error and Result types

#[cfg(feature = "render")]
use hayro::hayro_syntax;
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

#[cfg(feature = "render")]
impl From<hayro_syntax::LoadPdfError> for Error {
    fn from(_: hayro_syntax::LoadPdfError) -> Self {
        Self::LoadRenderer
    }
}

/// Error
#[derive(thiserror::Error, Debug, Clone)]
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
    #[cfg(feature = "render")]
    /// The document can not be loaded into the renderer engine.
    #[error("Load Renderer error.")]
    LoadRenderer,
    /// Renderer could not find the page.
    #[error("Renderer page not found.")]
    RendererPageNotFound,
    /// Renderer could not render into png.
    #[error("Renderer png error.")]
    RendererPng,
}

/// Result
pub type Result<T> = std::result::Result<T, Error>;
