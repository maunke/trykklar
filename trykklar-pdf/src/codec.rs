use crate::{Error, Result};
use lopdf::{Document, Object, ObjectId};

/// Resolves an object by providing the document, optional pdf object id and pdf object itself.
pub(crate) trait TryFromObject<'a>: Sized {
    /// Implements try from object.
    fn try_from_object(doc: &'a Document, id: Option<ObjectId>, obj: &'a Object) -> Result<Self>;
}

impl<'a, T: TryFromObject<'a>> TryFromObject<'a> for Vec<T> {
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        obj.as_array()
            .map_err(Error::from)?
            .iter()
            .map(|v| {
                let (id, o) = doc.dereference(v)?;
                T::try_from_object(doc, id, o)
            })
            .collect()
    }
}

pub(crate) trait IntoObject {
    fn into_object(self) -> Object;
}

pub(crate) trait ObjectAsF64 {
    fn as_f64(&self) -> Result<f64>;
}

impl ObjectAsF64 for Object {
    fn as_f64(&self) -> Result<f64> {
        Ok(self.as_float()? as f64)
    }
}

pub(crate) fn deref_f64<'a>(obj: &'a Object, doc: &'a Document) -> Result<f64> {
    doc.dereference(obj)?.1.as_f64()
}

pub(crate) fn deref_name<'a>(obj: &'a Object, doc: &'a Document) -> Result<&'a [u8]> {
    Ok(doc.dereference(obj)?.1.as_name()?)
}

pub(crate) fn deref_array<'a>(obj: &'a Object, doc: &'a Document) -> Result<&'a Vec<Object>> {
    Ok(doc.dereference(obj)?.1.as_array()?)
}
