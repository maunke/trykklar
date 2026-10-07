//! Dictionary Utilities

use crate::codec::{TryFromObject, TryIntoObject};
use crate::error::{Field, FieldError, OptionalField};
use crate::{Error, Pdf, Result};
use lopdf::{Dictionary, Object, ObjectId};

const PARENT_LIMIT: usize = 128;

/// Dictionary key entry
pub trait DictKey: Sized {
    /// Key name
    const KEY: &'static [u8];
    /// Inheritable attribute
    ///
    /// ISO 32000-1:2008 14.8.5.3 Attribute Values and Inheritance
    ///
    /// > Some attributes are defined as inheritable. Inheritable attributes propagate down the
    /// > structure tree; that is, an attribute that is specified for an element shall apply to all
    /// > the descendants of the element in the structure tree unless a descendent element specifies
    /// > an explicit value for the attribute.
    ///
    /// > An inheritable attribute may be specified for an element for the purpose of propagating
    /// > its value to child elements, even if the attribute is not meaningful for the parent
    /// > element. Non-inheritable attributes may be specified only for elements on which they would
    /// > be meaningful.
    ///
    /// In combination with [`read_field`] or [`read_optional_field`], these functions are using
    /// this field to look for parent dictionaries containing the [`Self::KEY`] when set to true.
    const INHERITABLE: bool = false;
}

pub(crate) fn write<T: DictKey + TryIntoObject>(
    entry: T,
    pdf: &mut Pdf,
    dict: impl FnOnce(&mut Pdf) -> Result<&mut Dictionary>,
) -> Result<()> {
    let obj = entry.try_into_object(pdf)?;
    dict(pdf)?.set(T::KEY, obj);
    Ok(())
}

pub(crate) fn exists<T: Into<ObjectId>>(id: T, pdf: &Pdf) -> Result<()> {
    Ok(pdf.doc().get_dictionary(id.into()).map(|_| ())?)
}

pub(crate) fn get_mut<T: Into<ObjectId>>(id: T, pdf: &mut Pdf) -> Result<&mut Dictionary> {
    Ok(pdf.doc_mut().get_dictionary_mut(id.into())?)
}

// Get a mutable dictionary by the parent id in combination with a dict key, or by the id directly.
pub(crate) fn get_mut_by_parent_id_or_key<'a>(
    parent_id: impl Into<ObjectId>,
    id: Option<impl Into<ObjectId>>,
    key: &'static [u8],
    pdf: &'a mut Pdf,
) -> Result<&'a mut Dictionary> {
    if let Some(id) = id {
        get_mut(id, pdf)
    } else {
        Ok(pdf
            .doc_mut()
            .get_dictionary_mut(parent_id.into())?
            .get_mut(key)?
            .as_dict_mut()?)
    }
}
fn read<'a, T: DictKey>(
    pdf: &'a Pdf,
    dict: &'a Dictionary,
    resolve: impl FnOnce(Option<ObjectId>, &'a Object) -> Result<T>,
) -> Field<T> {
    let Ok(obj) = dict.get(T::KEY) else {
        return Err(FieldError::Missing);
    };
    pdf.doc()
        .dereference(obj)
        .map_err(Error::from)
        .and_then(|(id, o)| resolve(id, o))
        .map_err(FieldError::Invalid)
}

fn into_optional<T>(field: Field<T>) -> OptionalField<T> {
    match field {
        Ok(t) => Some(Ok(t)),
        Err(FieldError::Missing) => None,
        Err(FieldError::Invalid(e)) => Some(Err(e)),
    }
}

/// Reads the field in a dictionary wrt. dereference and inheritance.
pub(crate) fn read_field<'a, T: DictKey + TryFromObject<'a>>(
    pdf: &'a Pdf,
    dict: &'a Dictionary,
) -> Field<T> {
    let mut dict = dict;
    for _ in 0..PARENT_LIMIT {
        match read(pdf, dict, |id, o| T::try_from_object(pdf, id, o)) {
            Err(FieldError::Missing) => (),
            field => return field,
        }
        if !T::INHERITABLE {
            return Err(FieldError::Missing);
        }
        let Ok(parent) = dict.get(b"Parent") else {
            return Err(FieldError::Missing);
        };
        let parent_id = parent.as_reference()?;
        dict = pdf.doc().get_dictionary(parent_id)?;
    }
    Err(FieldError::Invalid(Error::ParentLimit))
}

/// Reads the optional field in a dictionary wrt. dereference and inheritance.
pub(crate) fn read_optional_field<'a, T: DictKey + TryFromObject<'a>>(
    pdf: &'a Pdf,
    dict: &'a Dictionary,
) -> OptionalField<T> {
    into_optional(read_field::<T>(pdf, dict))
}

/// Reads the field in a dictionary with a custom resolver.
pub(crate) fn read_field_with_fn<'a, T: DictKey>(
    pdf: &'a Pdf,
    dict: &'a Dictionary,
    resolve: impl FnOnce(&'a Object) -> Result<T>,
) -> Field<T> {
    read(pdf, dict, |_, o| resolve(o))
}

/// Reads the optional field in a dictionary with a custom resolver.
pub(crate) fn read_optional_field_with_fn<'a, T: DictKey>(
    pdf: &'a Pdf,
    dict: &'a Dictionary,
    resolve: impl FnOnce(&'a Object) -> Result<T>,
) -> OptionalField<T> {
    into_optional(read_field_with_fn(pdf, dict, resolve))
}
