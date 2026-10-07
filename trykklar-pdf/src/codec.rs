use crate::{Error, Pdf, Result};
use lopdf::{Object, ObjectId};

/// Resolves an object by providing the document, optional pdf object id and pdf object itself.
pub(crate) trait TryFromObject<'a>: Sized {
    /// Implements try from object.
    fn try_from_object(pdf: &'a Pdf, id: Option<ObjectId>, obj: &'a Object) -> Result<Self>;
}

impl<'a, T: TryFromObject<'a>> TryFromObject<'a> for Vec<T> {
    fn try_from_object(pdf: &'a Pdf, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        obj.as_array()
            .map_err(Error::from)?
            .iter()
            .map(|v| {
                let (id, o) = pdf.doc().dereference(v)?;
                T::try_from_object(pdf, id, o)
            })
            .collect()
    }
}

pub(crate) trait TryIntoObject {
    fn try_into_object(self, pdf: &mut Pdf) -> Result<Object>;
}

pub(crate) trait ObjectAsF64 {
    fn as_f64(&self) -> Result<f64>;
}

impl ObjectAsF64 for Object {
    fn as_f64(&self) -> Result<f64> {
        Ok(self.as_float()? as f64)
    }
}

pub(crate) fn deref_f64<'a>(obj: &'a Object, pdf: &'a Pdf) -> Result<f64> {
    pdf.doc().dereference(obj)?.1.as_f64()
}

pub(crate) fn deref_name<'a>(obj: &'a Object, pdf: &'a Pdf) -> Result<&'a [u8]> {
    Ok(pdf.doc().dereference(obj)?.1.as_name()?)
}

pub(crate) fn deref_array<'a>(obj: &'a Object, pdf: &'a Pdf) -> Result<&'a Vec<Object>> {
    Ok(pdf.doc().dereference(obj)?.1.as_array()?)
}
