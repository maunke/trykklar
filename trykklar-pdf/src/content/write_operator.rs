//! Raw Content Stream Operation to write into content stream.
//! A subset of ISO 32000-1:2008 Table A.1 – PDF content stream operators
//! Use this to write a new operator into the content stream.

use crate::page::PdfPageMut;
use crate::{ImageId, Matrix, Pdf, Result};
use lopdf::content::Operation;
use lopdf::{Object, ObjectId};
use std::sync::Arc;

/// Operations
pub type WriteOperations<'a> = Vec<WriteOperation>;

/// A Write Operation from a present raw or a new operation
pub enum WriteOperation {
    /// A new operation
    New(Box<dyn TryIntoOperation>),
    /// Present operation read from a walker
    Raw(Arc<Operation>),
}

impl WriteOperation {
    /// Creates a [`Self::New`] write operation.
    pub fn new(value: impl TryIntoOperation + 'static) -> Self {
        Self::New(Box::new(value))
    }

    // This conversion consumes self in order to allow the raw operator to move into the operator if
    // the refcount is 1.
    pub(crate) fn try_into_operation(self, page: &mut PdfPageMut) -> Result<Operation> {
        match self {
            Self::New(op) => op.try_into_operation(page),
            Self::Raw(op) => Ok(Arc::unwrap_or_clone(op)),
        }
    }
}

/// Converts to an operation.
pub trait TryIntoOperation {
    /// Converts to an operation.
    fn try_into_operation(&self, page: &mut PdfPageMut) -> Result<Operation>;
}

pub(crate) trait OperatorName {
    const OPERATOR: &'static str;
}

#[derive(Debug, Clone, Copy)]
enum XObjectId {
    Image(ImageId),
}

impl From<XObjectId> for ObjectId {
    fn from(value: XObjectId) -> Self {
        match value {
            XObjectId::Image(id) => id.get(),
        }
    }
}

/// Paint [`crate::XObject`]
///
/// > `Do`: Invoke named XObject
pub struct PaintXObject {
    id: XObjectId,
}

impl PaintXObject {
    /// Creates [`PaintXObject`] by [`ImageId`].
    pub fn try_from_image(id: ImageId, pdf: &Pdf) -> Result<Self> {
        pdf.doc().get_object(id.into())?.as_stream()?;
        let id = XObjectId::Image(id);
        Ok(Self { id })
    }
}

impl OperatorName for PaintXObject {
    const OPERATOR: &'static str = "Do";
}

impl TryIntoOperation for PaintXObject {
    fn try_into_operation(&self, page: &mut PdfPageMut) -> Result<Operation> {
        let key = match self.id {
            XObjectId::Image(id) => format!("TRYKKLAR_IM_{}", id.get().0),
        };
        page.add_xobject(&key, self.id)?;
        let operands = vec![Object::Name(key.into())];
        Ok(Operation::new(Self::OPERATOR, operands))
    }
}

/// Modify the current transformation matrix
///
/// > `cm`: Concatenate matrix to current transformation matrix
pub struct ModifyCtm {
    matrix: Matrix,
}

impl ModifyCtm {
    /// Creates a new ctm modification.
    pub fn new(matrix: Matrix) -> Self {
        Self { matrix }
    }
}

impl OperatorName for ModifyCtm {
    const OPERATOR: &'static str = "cm";
}

impl TryIntoOperation for ModifyCtm {
    fn try_into_operation(&self, _page: &mut PdfPageMut) -> Result<Operation> {
        let operator = Self::OPERATOR;
        let Matrix { a, b, c, d, e, f } = self.matrix;
        let operands = vec![
            Object::Real(a as f32),
            Object::Real(b as f32),
            Object::Real(c as f32),
            Object::Real(d as f32),
            Object::Real(e as f32),
            Object::Real(f as f32),
        ];
        Ok(Operation::new(operator, operands))
    }
}
