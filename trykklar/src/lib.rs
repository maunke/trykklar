//! trykklar (Danish for 'ready-to-print')
//!
//! This crate serves as preflight focused layer on top of trykklar-pdf.
//!
//! It provides the high-level [`PageWalker`] that allows to let a set of [`WalkerProcessor`]
//! process all operations of a page content stream by traversing through form xobjects and tiling
//! patterns.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
mod error;
mod image;
mod trykklar;
mod walker;

pub extern crate trykklar_pdf as pdf;
pub use error::{Error, Result};
pub use image::{Dpi, Image};
pub use pdf::Pdf;
pub use trykklar::Trykklar;
pub use walker::{PageWalker, WalkerProcessor};
