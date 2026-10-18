//! Resource Dictionaries

use crate::codec::TryFromObject;
use crate::dict::DictKey;
use crate::{Error, Pdf, PdfPage, Result};
use lopdf::{Dictionary, Object, ObjectId};

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
#[derive(Debug, Clone, Copy)]
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
    fn try_from_object(_pdf: &'a Pdf, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        Ok(Self(obj.as_dict()?))
    }
}

/// Page Resources, chained by Parent.
#[derive(Debug, Clone)]
pub struct PageResources<'a>(Vec<Resources<'a>>);

impl<'a> TryFrom<&'a PdfPage<'a>> for PageResources<'a> {
    type Error = Error;
    fn try_from(page: &'a PdfPage<'a>) -> Result<Self> {
        let page_id = page.id();
        let doc = page.pdf().doc();
        let raw_page_resources = doc.get_page_resources(page_id.into())?;
        let mut resources = Vec::new();
        if let Some(resource) = raw_page_resources.0 {
            resources.push(Resources(resource));
        }
        for raw_page_resource_id in raw_page_resources.1 {
            let resource = doc.get_dictionary(raw_page_resource_id)?;
            resources.push(Resources(resource));
        }
        Ok(Self(resources))
    }
}

/// Resources the content can see:
///
/// - Page Resources
/// - Local: Form XObject can see either Local or Page Resources, Tiling Pattern can only see local
///   resources
/// - Empty: No resources
#[derive(Debug, Clone)]
pub enum ContentResources<'a> {
    /// Page Resources
    Page(PageResources<'a>),
    /// Local resources: Form XObject if present, or Tiling Pattern resources
    Local(Resources<'a>),
    /// No resources
    Empty,
}

impl<'a> ContentResources<'a> {
    /// Returns the resources dictionaries
    pub fn resource_dictionaries(&self) -> Vec<&'a Dictionary> {
        match self {
            Self::Local(r) => vec![r.get()],
            Self::Page(page) => page.0.iter().map(|p| p.get()).collect::<Vec<_>>(),
            Self::Empty => vec![],
        }
    }
}
