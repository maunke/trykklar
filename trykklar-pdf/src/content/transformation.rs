use crate::{Error, Pdf, Result};
use lopdf::ObjectId;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

type TransformationFn = dyn FnOnce(&mut Pdf) -> Result<()>;

/// A lazy transformation on an object.
pub struct Transformation {
    id: ObjectId,
    transform: Box<TransformationFn>,
}

impl PartialEq for Transformation {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Transformation {}
impl Hash for Transformation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Transformation {
    /// Create a new transformation that is performed later.
    pub fn new(
        id: impl Into<ObjectId>,
        transform: impl FnOnce(&mut Pdf) -> Result<()> + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            transform: Box::new(transform),
        }
    }

    fn run(self, pdf: &mut Pdf) -> Result<()> {
        let t = self.transform;
        t(pdf)
    }
}

/// A set of transformations.
pub struct Transformations(HashSet<Transformation>);

impl Default for Transformations {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformations {
    /// Initializes the set of transformations.
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// Inserts a new transformation and fails if a transformation with a given id exists in the
    /// set.
    pub fn insert(&mut self, value: Transformation) -> Result<()> {
        let id = value.id;
        match self.0.insert(value) {
            true => Ok(()),
            false => Err(Error::MultipleTransformationsOnObject(id)),
        }
    }

    /// Applies the all transformations in the set.
    pub fn run(self, pdf: &mut Pdf) -> Result<()> {
        for t in self.0.into_iter() {
            t.run(pdf)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformation_idempotence() -> Result<()> {
        let id = (1, 0);
        let transformation = Transformation::new(id, |_pdf: &mut Pdf| Ok(()));
        let mut transformations = Transformations::new();
        assert!(transformations.insert(transformation).is_ok());
        let transformation = Transformation::new(id, |_pdf: &mut Pdf| Ok(()));
        assert!(matches!(
            transformations.insert(transformation),
            Err(Error::MultipleTransformationsOnObject(..))
        ));
        Ok(())
    }
}
