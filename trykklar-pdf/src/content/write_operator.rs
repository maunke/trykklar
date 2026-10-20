//! Raw Content Stream Operation to write into content stream.
//! A subset of ISO 32000-1:2008 Table A.1 – PDF content stream operators
//! Use this to write a new operator into the content stream.

use crate::extgstate::ExtGStateId;
use crate::resources::ContentResourcesMut;
use crate::xobject::{FormXObjectId, FormXObjectMut, ImageXObjectMut};
use crate::{ImageId, Matrix, Result};
use lopdf::Object;
use lopdf::content::Operation;
use std::sync::Arc;

/// Operations
pub type WriteOperations = Vec<WriteOperation>;

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
    pub(crate) fn try_into_operation(
        self,
        resources: &mut ContentResourcesMut,
    ) -> Result<Operation> {
        match self {
            Self::New(mut op) => op.try_into_operation(resources),
            Self::Raw(op) => Ok(Arc::unwrap_or_clone(op)),
        }
    }
}

/// Converts to an operation.
pub trait TryIntoOperation {
    /// Converts to an operation.
    fn try_into_operation(&mut self, resources: &mut ContentResourcesMut) -> Result<Operation>;
}

pub(crate) trait OperatorName {
    const OPERATOR: &'static str;
}

/// Save Graphics State
pub struct SaveGraphicsState;

impl OperatorName for SaveGraphicsState {
    const OPERATOR: &'static str = "q";
}

impl TryIntoOperation for SaveGraphicsState {
    fn try_into_operation(&mut self, _resources: &mut ContentResourcesMut) -> Result<Operation> {
        Ok(Operation::new(Self::OPERATOR, vec![]))
    }
}

/// Restore Graphics State
pub struct RestoreGraphicsState;

impl OperatorName for RestoreGraphicsState {
    const OPERATOR: &'static str = "Q";
}

impl TryIntoOperation for RestoreGraphicsState {
    fn try_into_operation(&mut self, _resources: &mut ContentResourcesMut) -> Result<Operation> {
        Ok(Operation::new(Self::OPERATOR, vec![]))
    }
}

/// [`crate::content::Operator::SetExtGState`]
///
/// > `gs`: (PDF 1.2) Set parameters from graphics state parameter dictionary
pub struct SetExtGState(ExtGStateId);

impl From<ExtGStateId> for SetExtGState {
    fn from(value: ExtGStateId) -> Self {
        Self(value)
    }
}

impl OperatorName for SetExtGState {
    const OPERATOR: &'static str = "gs";
}

impl TryIntoOperation for SetExtGState {
    fn try_into_operation(&mut self, resources: &mut ContentResourcesMut) -> Result<Operation> {
        let key = format!("TRYKKLAR_EXTGSTATE_{}_{}", self.0.get().0, self.0.get().1);
        resources.add_ext_g_state(&key, self.0)?;
        let operands = vec![Object::Name(key.into())];
        Ok(Operation::new(Self::OPERATOR, operands))
    }
}

pub(crate) const PAINT_XOBJECT_OPERATOR: &str = "Do";

type ImageXObjectTransform = Option<Box<dyn FnOnce(&mut ImageXObjectMut) -> Result<()>>>;

/// Paint ImageXObject with optional transformation.
///
/// > `Do`: Invoke named XObject
pub struct PaintImageXObject {
    id: ImageId,
    transform: ImageXObjectTransform,
}

impl From<ImageId> for PaintImageXObject {
    fn from(id: ImageId) -> Self {
        Self {
            id,
            transform: None,
        }
    }
}

impl PaintImageXObject {
    /// Runs in [`TryIntoOperation`] the transformation on [`ImageXObjectMut`].
    pub fn with_transformation(
        mut self,
        value: impl FnOnce(&mut ImageXObjectMut) -> Result<()> + 'static,
    ) -> Self {
        self.transform = Some(Box::new(value));
        self
    }
}

impl OperatorName for PaintImageXObject {
    const OPERATOR: &'static str = PAINT_XOBJECT_OPERATOR;
}

impl TryIntoOperation for PaintImageXObject {
    fn try_into_operation(&mut self, resources: &mut ContentResourcesMut) -> Result<Operation> {
        let pdf_mut = resources.pdf_mut();
        if let Some(transform) = self.transform.take() {
            let mut image = ImageXObjectMut::new(pdf_mut, self.id);
            transform(&mut image)?;
        }
        let key = format!("TRYKKLAR_IM_{}_{}", self.id.get().0, self.id.get().1);
        resources.add_xobject(&key, self.id)?;
        let operands = vec![Object::Name(key.into())];
        Ok(Operation::new(Self::OPERATOR, operands))
    }
}

type FormXObjectTransform = Option<Box<dyn FnOnce(&mut FormXObjectMut) -> Result<()>>>;

/// Paint FormXObject with optional transformation.
///
/// > `Do`: Invoke named XObject
pub struct PaintFormXObject {
    id: FormXObjectId,
    transform: FormXObjectTransform,
}

impl PaintFormXObject {
    /// Creates [`PaintFormXObject`] by [`FormXObjectId`].
    pub fn new(id: FormXObjectId) -> Self {
        Self {
            id,
            transform: None,
        }
    }

    /// Runs in [`TryIntoOperation`] the transformation on [`FormXObjectMut`].
    pub fn with_transformation(
        mut self,
        value: impl FnOnce(&mut FormXObjectMut) -> Result<()> + 'static,
    ) -> Self {
        self.transform = Some(Box::new(value));
        self
    }
}

impl OperatorName for PaintFormXObject {
    const OPERATOR: &'static str = PAINT_XOBJECT_OPERATOR;
}

impl TryIntoOperation for PaintFormXObject {
    fn try_into_operation(&mut self, page: &mut ContentResourcesMut) -> Result<Operation> {
        if let Some(transform) = self.transform.take() {
            let mut image = FormXObjectMut::new(page.pdf_mut(), self.id);
            transform(&mut image)?;
        }
        let key = format!("TRYKKLAR_FORM_{}_{}", self.id.get().0, self.id.get().1);
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
    fn try_into_operation(&mut self, _resources: &mut ContentResourcesMut) -> Result<Operation> {
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
