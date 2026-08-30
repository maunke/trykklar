//! Dictionary Utilities

use crate::codec::{IntoObject, TryFromObject};
use crate::error::{Field, FieldError, OptionalField};
use crate::{Error, Result};
use lopdf::{Dictionary, Document, Object, ObjectId};

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

pub(crate) fn write<T: DictKey + IntoObject>(entry: T, dict: &mut Dictionary) {
    dict.set(T::KEY, entry.into_object());
}

fn read<'a, T: DictKey>(
    doc: &'a Document,
    dict: &'a Dictionary,
    resolve: impl FnOnce(Option<ObjectId>, &'a Object) -> Result<T>,
) -> Field<T> {
    let Ok(obj) = dict.get(T::KEY) else {
        return Err(FieldError::Missing);
    };
    doc.dereference(obj)
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
    doc: &'a Document,
    dict: &'a Dictionary,
) -> Field<T> {
    let mut dict = dict;
    for _ in 0..PARENT_LIMIT {
        match read(doc, dict, |id, o| T::try_from_object(doc, id, o)) {
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
        dict = doc.get_dictionary(parent_id)?;
    }
    Err(FieldError::Invalid(Error::ParentLimit))
}

/// Reads the optional field in a dictionary wrt. dereference and inheritance.
pub(crate) fn read_optional_field<'a, T: DictKey + TryFromObject<'a>>(
    doc: &'a Document,
    dict: &'a Dictionary,
) -> OptionalField<T> {
    into_optional(read_field::<T>(doc, dict))
}

/// Reads the field in a dictionary with a custom resolver.
pub(crate) fn read_field_with_fn<'a, T: DictKey>(
    doc: &'a Document,
    dict: &'a Dictionary,
    resolve: impl FnOnce(&'a Object) -> Result<T>,
) -> Field<T> {
    read(doc, dict, |_, o| resolve(o))
}

/// Reads the optional field in a dictionary with a custom resolver.
pub(crate) fn read_optional_field_with_fn<'a, T: DictKey>(
    doc: &'a Document,
    dict: &'a Dictionary,
    resolve: impl FnOnce(&'a Object) -> Result<T>,
) -> OptionalField<T> {
    into_optional(read_field_with_fn(doc, dict, resolve))
}
