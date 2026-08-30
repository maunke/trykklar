//! Error and Result types
use lopdf::ObjectId;
use std::ops::Deref;
use std::sync::Arc;

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(Arc::new(e))
    }
}

/// Error
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    /// Lopdf Error
    #[error("PDF error: {0}")]
    Lopdf(#[source] Arc<lopdf::Error>),
    /// IO Error
    #[error("IO error: {0}")]
    Io(#[source] Arc<std::io::Error>),
    /// Marks wrong object type parsing.
    #[error("wrong type: expected {expected:?}")]
    WrongObjectType {
        /// Expected object type.
        expected: ObjectTypeKind,
        /// Found object type.
        found: ObjectTypeKind,
    },
    /// Value is not finite.
    #[error("non finite value")]
    NonFinite,
    /// PDF page could not be found.
    #[error("Page not found")]
    PageNotFound,
    /// Page wide `/UserUnit` is not finite and not positive.
    #[error("invalid user unit {value}, should be finite and positive")]
    InvalidUserUnit {
        /// Parsed user unit value.
        value: f64,
    },
    /// In combination with `INHERITABLE` dict key the parent recursion lookup limit exceeded.
    #[error("parent limit exceeded")]
    ParentLimit,
    /// PDF object is invalid.
    #[error("invalid PDF object: {0}")]
    InvalidPdfObject(&'static str),
    /// PDF catalog could not be found.
    #[error("pdf catalog not found")]
    CatalogNotFound,
    /// The content stream operator is invalid.
    #[error("invalid operator")]
    InvalidOperator,
    /// The content stream operands are invalid.
    #[error("invalid operands")]
    InvalidOperands,
    /// The graphics stack (q/Q mechanism) is inbalanced.
    #[error("invalid graphics stack")]
    InvalidGraphicsStack,
    /// Invalid text rendering mode.
    #[error("invalid text rendering mode {value}")]
    InvalidTextRenderingMode {
        /// Parsed text rendering mode value.
        value: i64,
    },
    /// Undefined color space.
    #[error("undefined colorspace")]
    UndefinedColorSpace,
    /// Invalid color space.
    #[error("invalid colorspace")]
    InvalidColorSpace,
    /// Invalid ICC profile.
    #[error("invalid icc profile")]
    InvalidIccProfile,
    /// Invalid color space within a nested pattern.
    #[error("invalid colorspace with nested patterns")]
    InvalidColorSpaceNestedPattern,
    /// Content walker depth limit exceeded.
    #[error("max content walker depth reached")]
    ContentWalkerDepthExceeded,
    /// Invalid optional content stack.
    #[error("invalid oc stack")]
    InvalidOcStack,
    /// Specified resource could not be found.
    #[error("{kind:?} resource not found")]
    ResourceNotFound {
        /// Resource type.
        kind: ResourceKind,
    },
    /// Object cound not be found in PDF.
    #[error("object not found: {0:?}")]
    ObjectNotFound(ObjectId),
    /// Dereferencing recursion limit exceeded.
    #[error("reference limit reached")]
    ReferenceLimit,
    /// ExtGState not found.
    #[error("ExtGState key not found: {0:?}")]
    ExtGStateNotFound(Vec<u8>),
    /// Font not found.
    #[error("Font key not found: {0:?}")]
    FontNotFound(Vec<u8>),
    /// Required dictionary field not found.
    #[error("required field missing")]
    MissingField,
    /// Inline Images are not supported.
    #[error("inline images are unsupported")]
    InlineImageUnsupported,
    /// Invalid stream filter.
    #[error("invalid filter")]
    InvalidFilter,
    /// Explicit unsupported spec.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
}

/// Object type Kind
#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum ObjectTypeKind {
    /// Array
    #[error("Array")]
    Array,
    /// Unknown
    #[error("unknown: {0:?}")]
    Unknown(&'static str),
}

impl From<&'static str> for ObjectTypeKind {
    fn from(value: &'static str) -> Self {
        match value {
            "Array" => Self::Array,
            unknown => Self::Unknown(unknown),
        }
    }
}

impl From<lopdf::Error> for Error {
    fn from(value: lopdf::Error) -> Self {
        match value {
            lopdf::Error::ReferenceLimit => Self::ReferenceLimit,
            lopdf::Error::ObjectType { expected, found } => Self::WrongObjectType {
                expected: expected.into(),
                found: found.into(),
            },
            lopdf::Error::ObjectNotFound(object_id) => Self::ObjectNotFound(object_id),
            other => Self::Lopdf(Arc::new(other)),
        }
    }
}

/// Resource Kind
#[derive(Debug, Clone, Copy, Hash)]
pub enum ResourceKind {
    /// Font
    Font,
    /// XObject: Image or Form
    XObject,
    /// Pattern
    Pattern,
    /// ExtGState
    ExtGState,
    /// ColorSpace
    ColorSpace,
    /// Shading
    Shading,
    /// Optional content
    Oc,
    /// SoftMaskImage
    SoftMaskImage,
}

/// Field Error
#[derive(Debug, Clone, thiserror::Error)]
pub enum FieldError {
    /// Field is missing.
    #[error("field missing")]
    Missing,
    /// Field value is invalid.
    #[error(transparent)]
    Invalid(#[from] Error),
}

impl From<lopdf::Error> for FieldError {
    fn from(value: lopdf::Error) -> Self {
        Self::Invalid(value.into())
    }
}

/// Field Result
///
/// It abstracts the error into a Missing and an Invalid field.
pub type Field<T> = std::result::Result<T, FieldError>;
/// Optional Field Result
pub type OptionalField<T> = Option<std::result::Result<T, Error>>;

/// Result
pub type Result<T> = std::result::Result<T, Error>;

/// Result extension
pub trait ResultExt<T> {
    /// Maps as ref and error clone.
    fn ok_ref(&self) -> Result<&T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn ok_ref(&self) -> Result<&T> {
        self.as_ref().map_err(Clone::clone)
    }
}

impl From<FieldError> for Error {
    fn from(e: FieldError) -> Self {
        match e {
            FieldError::Missing => Error::MissingField,
            FieldError::Invalid(err) => err,
        }
    }
}

impl<T> ResultExt<T> for Field<T> {
    fn ok_ref(&self) -> Result<&T> {
        self.as_ref().map_err(|e| e.clone().into())
    }
}

pub(crate) trait ResultExtDeref<T: Deref> {
    fn ok_deref(&self) -> Result<&T::Target>;
}

impl<T: Deref> ResultExtDeref<T> for Result<T> {
    fn ok_deref(&self) -> Result<&T::Target> {
        self.as_deref().map_err(Clone::clone)
    }
}

pub(crate) trait FieldExt<T> {
    fn as_field_ref(&self) -> Field<&T>;
    fn as_result(&self) -> Result<T>;
}

impl<T: Clone> FieldExt<T> for Field<T> {
    fn as_field_ref(&self) -> Field<&T> {
        self.as_ref().map_err(Clone::clone)
    }
    fn as_result(&self) -> Result<T> {
        self.clone().map_err(Into::into)
    }
}

pub(crate) trait FieldExtDeref<T: Deref> {
    fn as_field_deref(&self) -> Field<&T::Target>;
}

impl<T: Deref> FieldExtDeref<T> for Field<T> {
    fn as_field_deref(&self) -> Field<&T::Target> {
        self.as_deref().map_err(Clone::clone)
    }
}

pub(crate) trait OptionalFieldExt<T> {
    fn as_field_ref(&self) -> OptionalField<&T>;
}

impl<T> OptionalFieldExt<T> for OptionalField<T> {
    fn as_field_ref(&self) -> OptionalField<&T> {
        self.as_ref().map(|r| r.as_ref().map_err(Clone::clone))
    }
}
