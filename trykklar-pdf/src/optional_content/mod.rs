//! Optional Content Group module containing types on document, page, resource and content stream
//! level.

mod oc_config;

use crate::codec::{TryFromObject, TryIntoObject};
use crate::dict::{self, DictKey, read_field, read_optional_field};
use crate::error::{
    Field, FieldError, FieldExtDeref, OptionalField, OptionalFieldExt, ResourceKind, ResultExt,
};
use crate::pdf::{CatalogId, Pdf};
use crate::{Error, Result, object_id};
use lopdf::{Dictionary, Document, Object, ObjectId, decode_text_string, text_string};
pub use oc_config::{BaseState, DOff, DOn, DOrder, DOrderItem, OcConfig, OcgGroup, OcgSubGroup};

object_id!(OCPropertiesId);

/// `/OCProperties` Optional Content Properties
///
/// ISO 32000-1:2008 7.7.2 Document Catalog Table 28 – Entries in the catalog dictionary
///
/// > (Optional; PDF 1.5; required if a document contains optional content) The document’s optional
/// > content properties dictionary (see 8.11.4, "Configuring Optional Content").
///
/// Find the supported fields implemented as methods accordingly to 8.11.4.2 Optional Content
/// Properties Dictionary, Table 100 – Entries in the Optional Content Properties Dictionary
pub struct OCProperties<'a> {
    pdf: &'a Pdf,
    id: Option<OCPropertiesId>,
    dict: &'a Dictionary,
}

impl<'a> OCProperties<'a> {
    /// Returns the optional ID.
    pub fn id(&self) -> Option<OCPropertiesId> {
        self.id
    }

    /// See [`Ocgs`]
    pub fn ocgs(&self) -> Field<Ocgs> {
        read_field::<Ocgs>(self.pdf, self.dict)
    }

    /// See [`OcConfig`]
    pub fn default_config(&self) -> Field<OcConfig> {
        read_field::<OcConfig>(self.pdf, self.dict)
    }
}

impl<'a> DictKey for OCProperties<'a> {
    const KEY: &'static [u8] = b"OCProperties";
}

impl<'a> TryFromObject<'a> for OCProperties<'a> {
    fn try_from_object(pdf: &'a Pdf, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        match obj {
            Object::Dictionary(dict) => Ok(Self {
                pdf,
                id: id.map(OCPropertiesId),
                dict,
            }),
            _ => Err(Error::InvalidPdfObject("OCProperties must be a dictionary")),
        }
    }
}

/// Mutation object for [`OCProperties`].
pub struct OCPropertiesMut<'a> {
    pdf: &'a mut Pdf,
    catalog_id: CatalogId,
    id: Option<OCPropertiesId>,
}

impl<'a> TryFrom<&'a mut Pdf> for OCPropertiesMut<'a> {
    type Error = Error;
    fn try_from(pdf: &'a mut Pdf) -> Result<Self> {
        let catalog = pdf.catalog()?;
        let catalog_id = catalog.id();
        let id = catalog
            .oc_properties()
            .ok_or(Error::OcPropertiesNotFound)??
            .id();

        Ok(Self {
            pdf,
            catalog_id,
            id,
        })
    }
}

impl<'a> OCPropertiesMut<'a> {
    /// Sets the default config.
    pub fn set_default_config(&mut self, oc_config: OcConfig) -> Result<()> {
        dict::write(oc_config, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut_by_parent_id_or_key(self.catalog_id, self.id, Self::KEY, pdf)
        })
    }
}

impl DictKey for OCPropertiesMut<'_> {
    const KEY: &'static [u8] = OCProperties::KEY;
}

/// `/OCGs` Array of optional content groups
///
/// ISO 32000-1:2008 8.11.4.2 Table 100 – Entries in the Optional Content Properties Dictionary
///
/// > (Required) An array of indirect references to all the optional content groups in the document
/// > (see 8.11.2, "Optional Content Groups"), in any order. Every optional content group shall be
/// > included in this array.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ocgs(Vec<Ocg>);

impl Ocgs {
    /// Returns the slice of containing OCGs.
    pub fn get(&self) -> &[Ocg] {
        &self.0
    }
}

impl DictKey for Ocgs {
    const KEY: &'static [u8] = b"OCGs";
}

impl<'a> TryFromObject<'a> for Ocgs {
    fn try_from_object(pdf: &'a Pdf, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        match obj {
            Object::Array(arr) => {
                let mut ocgs = vec![];
                for v in arr {
                    let ocg = pdf
                        .doc()
                        .dereference(v)
                        .map_err(Into::into)
                        .and_then(|(id, o)| Ocg::try_from_object(pdf, id, o))?;
                    ocgs.push(ocg);
                }
                Ok(Self(ocgs))
            }
            _ => Err(Error::InvalidPdfObject("OCGs must be an array")),
        }
    }
}

object_id!(OcgId);

/// ISO 32000-1:2008 8.11.2 Table 98 – Entries in an Optional Content Group Dictionary
///
/// > In its simplest form, each dictionary shall contain a Type entry and a Name for presentation
/// > in a user interface.
#[derive(Debug, Clone)]
pub struct Ocg {
    pub(crate) id: OcgId,
    pub(crate) name: Field<String>,
}

impl Ocg {
    /// Returns the ID.
    pub fn id(&self) -> OcgId {
        self.id
    }

    /// Returns the name of the optional content group.
    ///
    /// > (Required) The name of the optional content group, suitable for presentation in a reader’s
    /// > user interface.
    pub fn name(&self) -> Field<&str> {
        self.name.as_field_deref()
    }
}

impl<'a> TryFromObject<'a> for Ocg {
    fn try_from_object(pdf: &'a Pdf, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let id = match id {
            Some(id) => id,
            None => {
                return Err(Error::InvalidPdfObject("OCG object must be a reference"));
            }
        };

        match obj {
            Object::Dictionary(dict) => {
                let name = match dict.get_deref(b"Name", pdf.doc()) {
                    Ok(v) => decode_text_string(v).map_err(Into::into),
                    Err(..) => Err(FieldError::Missing),
                };
                Ok(Self {
                    id: OcgId(id),
                    name,
                })
            }
            _ => Err(Error::InvalidPdfObject("OCG must be a dictionary")),
        }
    }
}

impl TryIntoObject for Ocg {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(Object::Reference(self.id.into()))
    }
}

impl PartialEq for Ocg {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Ocg {}
impl std::hash::Hash for Ocg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

/// Mutation object for an [`Ocg`] by providing the [`crate::Pdf`] and [`OcgId`].
pub struct OcgMut<'a> {
    doc: &'a mut Document,
    id: OcgId,
}

impl<'a> OcgMut<'a> {
    /// Creates the mutation object for a given [`OcgId`].
    pub fn try_new(pdf: &'a mut Pdf, id: OcgId) -> Result<Self> {
        let doc = pdf.doc_mut();
        let mut ocg_mut = Self { doc, id };
        ocg_mut.dict_mut()?;
        Ok(ocg_mut)
    }

    fn dict_mut(&mut self) -> Result<&mut Dictionary> {
        match self.doc.get_object_mut(self.id.get())? {
            Object::Dictionary(dict) => Ok(dict),
            _ => Err(Error::InvalidPdfObject("OCG must be a dictionary")),
        }
    }

    /// Sets the name.
    ///
    /// If the name only contains ASCII characters, the string is encoded
    /// in PDFDocEncoding, otherwise in UTF-16BE.
    pub fn set_name(&mut self, name: &str) -> Result<()> {
        let dict = self.dict_mut()?;
        dict.set("Name", text_string(name));
        Ok(())
    }
}

/// An inline OCG defined as direct object in content stream.
///
/// ISO 3200-1:2008 14.6.2 Property Lists
///
/// > If all of the values in a property list dictionary are direct objects, the dictionary may be
/// > written inline in the content stream as a direct object. If any of the values are indirect
/// > references to objects outside the content stream, the property list dictionary shall be
/// > defined as a named resource in the Properties subdictionary of the current resource dictionary
/// > (see 7.8.3, “Resource Dictionaries”) and referenced by name as the properties operand of the
/// > DP or BDC operator.
///
/// This describes the BDC definition [`crate::content::Operator::BeginMarkedContent`] to have the
/// property list dictionary that may be written inline.
///
/// In contrast to, in 8.11.3.2 "Optional Content in Content Streams" it stands
///
/// > The property list associated with the marked content shall specify either an optional content
/// > group or optional content membership dictionary to which the content belongs. Because a group
/// > shall be an indirect object and a membership dictionary contains references to indirect
/// > objects, the property list shall be a named resource listed in the Properties subdictionary of
/// > the current resource dictionary (see 14.6.2, "Property Lists"), as shown in EXAMPLE 1 and
/// > EXAMPLE 2 in this sub-clause
///
/// such that the inline OCG is a lenient way to allow the parsing of direct named objects in order
/// to reject or fix it afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InlineOcg {
    pub(crate) name: String,
}

impl InlineOcg {
    /// Returns the name of the inline OCG.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Optional Content
///
/// The Optional Content can be referenced from
///
/// - content operator [`crate::Operator::BeginMarkedContent`]
/// - XObjects [`crate::xobject::XObject`], namely [`crate::xobject::FormXObject`] and
///   [`crate::xobject::ImageXObject`]
///
/// ISO 32000-1:2008 8.11.3.2 Optional Content in Content Streams
///
/// > Sections of content in a content stream (including a page's Contents stream, a form or
/// > pattern’s content stream, glyph descriptions a Type 3 font as specified by its CharProcs
/// > entry, or an annotation’s appearance) may be made optional by enclosing them between the
/// > marked-content operators BDC and EMC (see 14.6, "Marked Content") with a marked-content tag of
/// > OC. In addition, a DP marked-content operator may be placed in a page’s content stream to
/// > force a reference to an optional content group or groups on the page, even when the page has
/// > no current content in that layer.
///
/// > The property list associated with the marked content shall specify either an optional content
/// > group or optional content membership dictionary to which the content belongs. Because a group
/// > shall be an indirect object and a membership dictionary contains references to indirect
/// > objects, the property list shall be a named resource listed in the Properties subdictionary of
/// > the current resource dictionary (see 14.6.2, "Property Lists"), as shown in EXAMPLE 1 and
/// > EXAMPLE 2 in this sub-clause.
///
/// ISO 32000-1:2008 8.11.3.3 Optional Content in XObjects and Annotations
///
/// `OC` Entry
///
/// > In addition to marked content within content streams, form XObjects and image XObjects (see
/// > 8.8, "External Objects") and annotations (see 12.5, "Annotations") may contain an OC entry,
/// > which shall be an optional content group or an optional content membership dictionary.
///
/// This covers all three kind of optional content referennces.
#[derive(Debug, Clone)]
pub enum Oc {
    /// Optional content group.
    Ocg(Ocg),
    /// Optional content membership dictionary.
    Ocmd(Ocmd),
    /// Inline OCG (lenient parsing, see [`InlineOcg`])
    ///
    /// This originates from property lists in context of marked content operators as DP and BDC.
    /// For simplicity it also be allowed to be parsed in general, when a the named entry is not an
    /// indirect reference.
    InlineOcg(InlineOcg),
}

impl DictKey for Oc {
    const KEY: &'static [u8] = b"OC";
}

impl Oc {
    pub(crate) fn resolve(
        properties: &Object,
        resource_dicts: &[&Dictionary],
        pdf: &Pdf,
    ) -> Result<Self> {
        let (id, dict) = match properties {
            Object::Name(name) => {
                for rd in resource_dicts {
                    if let Ok(Object::Dictionary(props)) = rd.get_deref(b"Properties", pdf.doc())
                        && let Ok(entry) = props.get(name)
                    {
                        let (id, obj) = pdf.doc().dereference(entry)?;
                        return Self::from_dict(id, obj.as_dict()?, pdf);
                    }
                }
                return Err(Error::ResourceNotFound {
                    kind: ResourceKind::Oc,
                });
            }
            other => {
                let (id, obj) = pdf.doc().dereference(other)?;
                (id, obj.as_dict()?)
            }
        };

        Self::from_dict(id, dict, pdf)
    }

    fn from_dict(id: Option<ObjectId>, dict: &Dictionary, pdf: &Pdf) -> Result<Self> {
        let is_ocmd = matches!(
            dict.get(b"Type").and_then(Object::as_name).ok(),
            Some(b"OCMD")
        ) || dict.has(b"OCGs");

        if !is_ocmd {
            if id.is_none() {
                let name_obj = dict.get_deref(b"Name", pdf.doc())?;
                let name = decode_text_string(name_obj)?;
                return Ok(Oc::InlineOcg(InlineOcg { name }));
            }
            let ocg = Ocg::try_from_object(pdf, id, &Object::Dictionary(dict.clone()))?;
            return Ok(Oc::Ocg(ocg));
        }

        let ocgs = read_optional_field(pdf, dict);
        let policy = match read_optional_field(pdf, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };

        Ok(Oc::Ocmd(Ocmd { ocgs, policy }))
    }
}
/// ISO 32000-1:2008 8.11.2.2 Table 99 – Entries in an Optional Content Membership Dictionary
///
/// > An optional content membership dictionary may express its visibility policy in two ways:
/// > - The P entry may specify a simple boolean expression indicating how the optional content
/// > groups specified by the OCGs entry determine the visibility of content controlled by the
/// > membership dictionary.
/// > - PDF 1.6 introduced the VE entry, which is a visibility expression that may be used to
/// > specify an arbitrary boolean expression for computing the visibility of content from the
/// > states of optional content groups.
#[derive(Debug, Clone)]
pub struct Ocmd {
    ocgs: OptionalField<Ocgs>,
    policy: Result<OcmdPolicy>,
}

impl Ocmd {
    /// Returns the optional array of Ocg.
    pub fn ocgs(&self) -> OptionalField<&Ocgs> {
        self.ocgs.as_field_ref()
    }

    /// Returns the visibility policy.
    pub fn policy(&self) -> Result<&OcmdPolicy> {
        self.policy.ok_ref()
    }
}

/// `P` Visibility Policy for Content
///
/// > (Optional) A name specifying the visibility policy for content belonging to this membership
/// > dictionary.
///
/// > Default value: AnyOn
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum OcmdPolicy {
    /// > AllOn visible only if all of the entries in OCGs are ON
    AllOn,
    #[default]
    /// > AnyOn visible if any of the entries in OCGs are ON
    AnyOn,
    /// > AnyOff visible if any of the entries in OCGs are OFF
    AnyOff,
    /// > AllOff visible only if all of the entries in OCGs are OFF
    AllOff,
}

impl DictKey for OcmdPolicy {
    const KEY: &'static [u8] = b"P";
}

impl TryFromObject<'_> for OcmdPolicy {
    fn try_from_object(_pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let policy = match obj.as_name()? {
            b"AllOn" => Self::AllOn,
            b"AnyOn" => Self::AnyOn,
            b"AnyOff" => Self::AnyOff,
            b"AllOff" => Self::AllOff,
            _ => {
                return Err(Error::InvalidPdfObject(
                    "Ocmd Policy must be one of AllOn, AnyOn, AnyOff, AllOff",
                ));
            }
        };
        Ok(policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oc_properties_ocgs() -> Result<()> {
        let pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;

        let names_test = [
            "Images",
            "RGB",
            "Grayscale",
            "Languages",
            "English",
            "German",
        ];

        pdf.catalog()?
            .oc_properties()
            .unwrap()?
            .ocgs()
            .unwrap()
            .get()
            .iter()
            .zip(names_test.iter())
            .for_each(|(ocg, name_test)| {
                assert_eq!(&ocg.name().unwrap(), name_test);
            });
        Ok(())
    }

    #[test]
    fn oc_properties_group_ocg() -> Result<()> {
        let pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let d = pdf.catalog()?.oc_properties().unwrap()?.default_config()?;

        let order = d.order.unwrap();
        assert_eq!(order.get().len(), 2);

        struct Group {
            name: String,
            ocgs: Vec<String>,
        }

        let test_groups = [
            Group {
                name: "Images".into(),
                ocgs: vec!["Grayscale".into(), "RGB".into()],
            },
            Group {
                name: "Languages".into(),
                ocgs: vec!["English".into(), "German".into()],
            },
        ];

        order
            .get()
            .iter()
            .zip(test_groups.iter())
            .for_each(|(item, test_group)| {
                let DOrderItem::OcgSubGroup(group) = item else {
                    panic!("item should be a sub group")
                };
                assert_eq!(group.header().name().expect("must be ok"), test_group.name);
                group.body().iter().zip(test_group.ocgs.iter()).for_each(
                    |(body_item, test_ocg_name)| {
                        let DOrderItem::Ocg(ocg_item) = body_item else {
                            panic!("body item must be an ocg")
                        };
                        assert_eq!(ocg_item.name().expect("must be ok"), test_ocg_name);
                    },
                );
            });

        Ok(())
    }

    #[test]
    fn rename_ocgs() -> Result<()> {
        let mut pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let names = [
            "Images",
            "RGB",
            "Grayscale",
            "Languages",
            "English",
            "German",
        ];
        pdf.catalog()?
            .oc_properties()
            .unwrap()?
            .ocgs()
            .unwrap()
            .get()
            .iter()
            .zip(names.iter())
            .for_each(|(ocg, name_test)| {
                assert_eq!(&ocg.name().unwrap(), name_test);
            });

        // rename ocgs
        let new_names = names.map(|n| n.to_uppercase());
        for (ocg, new_name) in pdf
            .catalog()?
            .oc_properties()
            .unwrap()?
            .ocgs()
            .unwrap()
            .get()
            .iter()
            .zip(new_names.iter())
        {
            let mut ocg_mut = OcgMut::try_new(&mut pdf, ocg.id())?;
            ocg_mut.set_name(new_name)?;
        }

        // check renamed ocgs
        pdf.catalog()?
            .oc_properties()
            .unwrap()?
            .ocgs()
            .unwrap()
            .get()
            .iter()
            .zip(new_names.iter())
            .for_each(|(ocg, name_test)| {
                assert_eq!(&ocg.name().unwrap(), name_test);
            });
        Ok(())
    }
}
