//! trykklar-pdf (Danish for 'ready-to-print')
//!
//! This crate follows the ISO 32000-1 specification. The [`ContentWalker`] represents one central
//! feature that allows a consumer to traverse through the page, form xobject and tiling pattern
//! content stream. Each [`Operator`] is parsed in a lenient way in order traverse through the
//! content streams without hard failing, as required for preflight. The [`GraphicsState`] follows
//! the lenient parsing philosophy.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
mod codec;
pub mod color;
pub mod content;
mod dict;
pub mod error;
pub mod extgstate;
pub mod font;
pub mod geometry;
pub mod id;
pub mod matrix;
pub mod ocg;
pub mod page;
pub mod pattern;
pub mod pdf;
pub mod resources;
pub mod stream;
pub mod text;
pub mod unit;
pub mod xobject;

pub(crate) use codec::ObjectAsF64;
pub use color::{Color, ColorSpace, PatternColor};
pub use content::{ContentWalker, ContentWalkerStep, GraphicsState, Operator, WalkerContext};
pub use error::{Error, Result};
pub use extgstate::{BlendMode, SoftMask};
pub use geometry::{BBox, Rect};
pub(crate) use id::object_id;
pub use matrix::Matrix;
pub use ocg::{
    BaseState, DOff, DOn, DOrder, DOrderItem, InlineOcg, OCProperties, Oc, OcConfig, Ocg, OcgGroup,
    OcgId, OcgSubGroup, Ocgs,
};
pub use page::PdfPage;
pub use pattern::{Pattern, ShadingPattern, TilingPattern};
pub use pdf::Pdf;
pub use unit::{Inch, Length, Mm, PhysicalUnit, Pt, UserSpace, UserUnit};
pub use xobject::{FormXObject, ImageId, ImageKind, ImageXObject, XObject};
