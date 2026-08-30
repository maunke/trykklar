//! Resource Dictionaries

use crate::Result;
use crate::codec::TryFromObject;
use crate::dict::DictKey;
use lopdf::{Dictionary, Document, Object, ObjectId};

/// Resource Dictionary
///
/// ISO 32000-1:2008 7.8.3 Resource Dictionaries
///
/// > As stated above, the operands supplied to operators in a content stream shall only be direct
/// > objects; indirect objects and object references shall not be permitted. In some cases, an
/// > operator shall refer to a PDF object that is defined outside the content stream, such as a
/// > font dictionary or a stream containing image data. This shall be accomplished by defining such
/// > objects as named resources and referring to them by name from within the content stream.
/// >
/// > Named resources shall be meaningful only in the context of a content stream. The scope of a
/// > resource name shall be local to a particular content stream and shall be unrelated to
/// > externally known identifiers for objects such as fonts. References from one object outside of
/// > content streams to another outside of content streams shall be made by means of indirect
/// > object references rather than named resources.
///
/// The lookup for named objects in the resource dictionaries is left to the object type resolvers,
/// such that this struct only contains the dictionary.
#[derive(Debug, Clone)]
pub struct Resources<'a>(&'a Dictionary);

impl<'a> Resources<'a> {
    /// Returns the dictionary.
    pub fn get(&self) -> &'a Dictionary {
        self.0
    }
}

impl DictKey for Resources<'_> {
    const KEY: &'static [u8] = b"Resources";
}

impl<'a> TryFromObject<'a> for Resources<'a> {
    fn try_from_object(_doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        Ok(Self(obj.as_dict()?))
    }
}
