//! Optional Content Group module containing types on document, page, resource and content stream
//! level.

use crate::codec::TryFromObject;
use crate::dict::{DictKey, read_field, read_optional_field};
use crate::error::{
    Field, FieldError, FieldExtDeref, OptionalField, OptionalFieldExt, ResourceKind, ResultExt,
};
use crate::pdf::Pdf;
use crate::{Error, Result, object_id};
use lopdf::{Dictionary, Document, Object, ObjectId, decode_text_string, text_string};
use std::collections::HashSet;

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
    doc: &'a Document,
    dict: &'a Dictionary,
}

impl<'a> OCProperties<'a> {
    /// See [`Ocgs`]
    pub fn ocgs(&self) -> Field<Ocgs> {
        read_field::<Ocgs>(self.doc, self.dict)
    }

    /// See [`OcConfig`]
    pub fn default_config(&self) -> Field<OcConfig> {
        read_field::<OcConfig>(self.doc, self.dict)
    }
}

impl<'a> DictKey for OCProperties<'a> {
    const KEY: &'static [u8] = b"OCProperties";
}

impl<'a> TryFromObject<'a> for OCProperties<'a> {
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        match obj {
            Object::Dictionary(dict) => Ok(Self { doc, dict }),
            _ => Err(Error::InvalidPdfObject("OCProperties must be a dictionary")),
        }
    }
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
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        match obj {
            Object::Array(arr) => {
                let mut ocgs = vec![];
                for v in arr {
                    let ocg = doc
                        .dereference(v)
                        .map_err(Into::into)
                        .and_then(|(id, o)| Ocg::try_from_object(doc, id, o))?;
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
    fn try_from_object(doc: &'a Document, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let id = match id {
            Some(id) => id,
            None => {
                return Err(Error::InvalidPdfObject("OCG object must be a reference"));
            }
        };

        match obj {
            Object::Dictionary(dict) => {
                let name = match dict.get_deref(b"Name", doc) {
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

/// `/D` Optional Content Config
///
/// ISO 32000-1:2008 8.11.4.2 Table 100 – Entries in the Optional Content Properties Dictionary
///
/// > (Required) The default viewing optional content configuration dictionary (see 8.11.4.3,
/// > "Optional Content Configuration Dictionaries").
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > The D and Configs entries in Table 100 are configuration dictionaries, which represent
/// > different presentations of a document’s optional content groups for use by conforming readers.
/// > The D configuration dictionary shall be used to specify the initial state of the optional
/// > content groups when a document is first opened. Configs lists other configurations that may be
/// > used under particular circumstances. The entries in a configuration dictionary are shown in
/// > Table 101.
#[derive(Debug, Clone)]
pub struct OcConfig {
    pub(crate) base_state: Result<BaseState>,
    pub(crate) on: Result<DOn>,
    pub(crate) off: Result<DOff>,
    pub(crate) order: Result<DOrder>,
}

impl DictKey for OcConfig {
    const KEY: &'static [u8] = b"D";
}

impl OcConfig {
    /// Create an [`OcConfig`] by providing the base state, on / off array and the order.
    pub fn new(base_state: BaseState, on: DOn, off: DOff, order: DOrder) -> Self {
        Self {
            base_state: Ok(base_state),
            on: Ok(on),
            off: Ok(off),
            order: Ok(order),
        }
    }

    /// Returns the [`BaseState`].
    pub fn base_state(&self) -> Result<BaseState> {
        self.base_state.ok_ref().copied()
    }

    /// Returns an array of OCG with state on, [`DOn`].
    pub fn on(&self) -> Result<&DOn> {
        self.on.ok_ref()
    }

    /// Returns an array of OCG with state off, [`DOff`].
    pub fn off(&self) -> Result<&DOff> {
        self.off.ok_ref()
    }

    /// Returns th order for the representation of Ocgs.
    pub fn order(&self) -> Result<&DOrder> {
        self.order.ok_ref()
    }

    fn all_visible(&self, ocgs: &Ocgs) -> Result<bool> {
        for ocg in ocgs.get() {
            if !self.ocg_visible(ocg)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn any_visible(&self, ocgs: &Ocgs) -> Result<bool> {
        for ocg in ocgs.get() {
            if self.ocg_visible(ocg)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Checks the visibilty of an [`Ocg`].
    pub fn ocg_visible(&self, ocg: &Ocg) -> Result<bool> {
        // Check ordering: Off -> On -> BaseState
        let visible = if self.off.ok_ref()?.get().contains(ocg) {
            false
        } else if self.on.ok_ref()?.get().contains(ocg) {
            true
        } else {
            self.base_state.ok_ref()?.visible()
        };
        Ok(visible)
    }

    /// Checks the visibilty of an [`Oc`].
    pub fn oc_visible(&self, oc: &Oc) -> Result<bool> {
        match oc {
            Oc::Ocg(ocg) => self.ocg_visible(ocg),
            Oc::InlineOcg(_) => Ok(self.base_state.ok_ref()?.visible()),
            Oc::Ocmd(ocmd) => {
                let ocgs = &ocmd.ocgs;
                if ocgs.is_none() {
                    return Ok(true);
                }
                match ocgs {
                    None => Ok(true),
                    Some(ocgs) => match &ocmd.policy.ok_ref()? {
                        OcmdPolicy::AllOn => self.all_visible(ocgs.ok_ref()?),
                        OcmdPolicy::AnyOn => self.any_visible(ocgs.ok_ref()?),
                        OcmdPolicy::AnyOff => self.all_visible(ocgs.ok_ref()?).map(|v| !v),
                        OcmdPolicy::AllOff => self.any_visible(ocgs.ok_ref()?).map(|v| !v),
                    },
                }
            }
        }
    }
}

impl<'a> TryFromObject<'a> for OcConfig {
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let Object::Dictionary(dict) = obj else {
            return Err(Error::InvalidPdfObject(
                "configuration must be a dictionary",
            ));
        };
        let base_state = match read_optional_field(doc, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        let on = match read_optional_field(doc, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        let off = match read_optional_field(doc, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        let order = match read_optional_field(doc, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        Ok(Self {
            base_state,
            on,
            off,
            order,
        })
    }
}

/// `/ON` Array of OCG with state on
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) An array of optional content groups whose state shall be set to ON when this
/// > configuration is applied. If the BaseState entry is ON, this entry is redundant.
#[derive(Debug, Clone, Default)]
pub struct DOn(HashSet<Ocg>);

impl DOn {
    /// Returns the set of [`Ocg`].
    pub fn get(&self) -> &HashSet<Ocg> {
        &self.0
    }
}

impl FromIterator<Ocg> for DOn {
    fn from_iter<I: IntoIterator<Item = Ocg>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl DictKey for DOn {
    const KEY: &'static [u8] = b"ON";
}

impl TryFromObject<'_> for DOn {
    fn try_from_object(doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let Object::Array(arr) = obj else {
            return Err(Error::InvalidPdfObject("D On must be an array"));
        };
        let mut ocg_set = HashSet::new();
        for element_obj in arr {
            let (ocg_id, ocg_obj) = doc.dereference(element_obj)?;
            let ocg = Ocg::try_from_object(doc, ocg_id, ocg_obj)?;
            ocg_set.insert(ocg);
        }
        Ok(Self(ocg_set))
    }
}

/// `/OFF` Array of OCG with state off
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) An array of optional content groups whose state shall be set to OFF when this
/// > configuration is applied. If the BaseState entry is OFF, this entry is redundant.
#[derive(Debug, Clone, Default)]
pub struct DOff(HashSet<Ocg>);

impl DOff {
    /// Returns the set of [`Ocg`].
    pub fn get(&self) -> &HashSet<Ocg> {
        &self.0
    }
}

impl FromIterator<Ocg> for DOff {
    fn from_iter<I: IntoIterator<Item = Ocg>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl DictKey for DOff {
    const KEY: &'static [u8] = b"OFF";
}

impl TryFromObject<'_> for DOff {
    fn try_from_object(doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let Object::Array(arr) = obj else {
            return Err(Error::InvalidPdfObject("D Off must be an array"));
        };
        let mut ocg_set = HashSet::new();
        for element_obj in arr {
            let (ocg_id, ocg_obj) = doc.dereference(element_obj)?;
            let ocg = Ocg::try_from_object(doc, ocg_id, ocg_obj)?;
            ocg_set.insert(ocg);
        }
        Ok(Self(ocg_set))
    }
}

/// An optionally named group of [`DOrderItem`].
#[derive(Debug, Clone)]
pub struct OcgGroup {
    name: Option<String>,
    items: Vec<DOrderItem>,
}

impl OcgGroup {
    /// Returns the name of the group.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the slice of [`DOrderItem`] in the group.
    pub fn items(&self) -> &[DOrderItem] {
        &self.items
    }
}

/// A group of OCGs with an Ocg as header.
#[derive(Debug, Clone)]
pub struct OcgSubGroup {
    header: Ocg,
    body: Vec<DOrderItem>,
}

impl OcgSubGroup {
    /// Returns the header Ocg of the group.
    pub fn header(&self) -> &Ocg {
        &self.header
    }

    /// Returns the body of the group as slice of [`DOrderItem`].
    pub fn body(&self) -> &[DOrderItem] {
        &self.body
    }
}

/// Find the definition in the parent object [`DOrder`].
#[derive(Debug, Clone)]
pub enum DOrderItem {
    /// Optional content group [`Ocg`].
    Ocg(Ocg),
    /// Array of an optional content group [`Ocg`] with optional header name.
    OcgGroup(OcgGroup),
    /// Array of an optional content group [`Ocg`] with an Ocg as header.
    OcgSubGroup(OcgSubGroup),
}

/// `/Order` Array of [`DOrderItem`]
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) An array specifying the order for presentation of optional content groups in a
/// > conforming reader’s user interface. The array elements may include the following objects:
/// >
/// > Optional content group dictionaries, whose Name entry shall be displayed in the user interface
/// > by the conforming reader.
/// >
/// > Arrays of optional content groups which may be displayed by a
/// > conforming reader in a tree or outline structure. Each nested array may optionally have as its
/// > first element a text string to be used as a non-selectable label in a conforming reader’s user
/// > interface
/// >
/// > Text labels in nested arrays shall be used to present collections of related
/// > optional content groups, and not to communicate actual nesting of content inside multiple
/// > layers of groups (see EXAMPLE 1 in 8.11.4.3, "Optional Content Configuration Dictionaries").
/// > To reflect actual nesting of groups in the content, such as for layers with sublayers, nested
/// > arrays of groups without a text label shall be used (see EXAMPLE 2 in 8.11.4.3, "Optional
/// > Content Configuration Dictionaries").
/// >
/// > An empty array [] explicitly specifies that no groups shall be presented.
/// >
/// > In the default configuration dictionary, the default value shall be an empty array; in other
/// > configuration dictionaries, the default shall be the Order value from the default
/// > configuration dictionary.
///
/// > Any groups not listed in this array shall not be presented in any user interface that uses the
/// > configuration.
#[derive(Debug, Clone, Default)]
pub struct DOrder(Vec<DOrderItem>);

impl DOrder {
    /// Returns the array of [`DOrderItem`].
    pub fn get(&self) -> &[DOrderItem] {
        &self.0
    }
}

impl DictKey for DOrder {
    const KEY: &'static [u8] = b"Order";
}

/// This is the recursive resolving of arrays of DOrderItem as defined in [`DOrder`].
fn d_order_from_array<'a>(
    doc: &'a Document,
    objects: &'a [Object],
    top_level: bool,
) -> Result<Vec<DOrderItem>> {
    let mut group = OcgGroup {
        name: None,
        items: vec![],
    };
    let mut items: Vec<DOrderItem> = vec![];
    for (idx, obj) in objects.iter().enumerate() {
        if idx == 0
            && let Ok(text) = decode_text_string(obj)
        {
            group.name = Some(text);
        } else {
            let obj = doc.dereference(obj)?;
            match obj {
                (_, Object::Array(arr)) => {
                    let top_level = matches!(group.items.last(), Some(DOrderItem::Ocg(..)));
                    let order_items = d_order_from_array(doc, arr, top_level)?;
                    if let Some(DOrderItem::Ocg(ocg)) = group.items.last() {
                        let sub_group = DOrderItem::OcgSubGroup(OcgSubGroup {
                            header: ocg.clone(),
                            body: order_items.clone(),
                        });
                        group.items.pop();
                        group.items.push(sub_group.clone());
                    } else {
                        group.items.extend(order_items);
                    }
                }
                (id, v) => {
                    let ocg = Ocg::try_from_object(doc, id, v)?;
                    group.items.push(DOrderItem::Ocg(ocg));
                }
            }
        }
    }
    if !group.items().is_empty() {
        if top_level {
            items.extend(group.items);
        } else {
            items.push(DOrderItem::OcgGroup(group.clone()));
        }
    }

    Ok(items)
}

impl<'a> TryFromObject<'a> for DOrder {
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        match obj {
            Object::Array(array) => Ok(Self(d_order_from_array(doc, array, true)?)),
            _ => Err(Error::InvalidPdfObject("DOrder must be an array")),
        }
    }
}

/// `/BaseState` Initial State of all OCGs
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) Used to initialize the states of all the optional content groups in a document when
/// > this configuration is applied.
/// >
/// > After this initialization, the contents of the ON and OFF arrays shall be processed,
/// > overriding the state of the groups included in the arrays. Default value: ON. If BaseState is
/// > present in the document’s default configuration dictionary, its value shall be ON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseState {
    /// `ON`
    ///
    /// > The states of all groups shall be turned ON.
    #[default]
    On,
    /// `OFF`
    ///
    /// > The states of all groups shall be turned OFF.
    Off,
    /// `Unchanged`
    ///
    /// > The states of all groups shall be left unchanged.
    Unchanged,
}

impl BaseState {
    fn visible(&self) -> bool {
        match self {
            Self::On => true,
            Self::Off => false,
            Self::Unchanged => true,
        }
    }
}

impl DictKey for BaseState {
    const KEY: &'static [u8] = b"BaseState";
}

impl<'a> TryFromObject<'a> for BaseState {
    fn try_from_object(_doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let base_state = match obj.as_name()? {
            b"ON" => Self::On,
            b"OFF" => Self::Off,
            b"Unchanged" => Self::Unchanged,
            _ => {
                return Err(Error::InvalidPdfObject(
                    "BaseState must be one of ON, OFF and Unchanged",
                ));
            }
        };
        Ok(base_state)
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
        doc: &Document,
    ) -> Result<Self> {
        let (id, dict) = match properties {
            Object::Name(name) => {
                for rd in resource_dicts {
                    if let Ok(Object::Dictionary(props)) = rd.get_deref(b"Properties", doc)
                        && let Ok(entry) = props.get(name)
                    {
                        let (id, obj) = doc.dereference(entry)?;
                        return Self::from_dict(id, obj.as_dict()?, doc);
                    }
                }
                return Err(Error::ResourceNotFound {
                    kind: ResourceKind::Oc,
                });
            }
            other => {
                let (id, obj) = doc.dereference(other)?;
                (id, obj.as_dict()?)
            }
        };

        Self::from_dict(id, dict, doc)
    }

    fn from_dict(id: Option<ObjectId>, dict: &Dictionary, doc: &Document) -> Result<Self> {
        let is_ocmd = matches!(
            dict.get(b"Type").and_then(Object::as_name).ok(),
            Some(b"OCMD")
        ) || dict.has(b"OCGs");

        if !is_ocmd {
            if id.is_none() {
                let name_obj = dict.get_deref(b"Name", doc)?;
                let name = decode_text_string(name_obj)?;
                return Ok(Oc::InlineOcg(InlineOcg { name }));
            }
            let ocg = Ocg::try_from_object(doc, id, &Object::Dictionary(dict.clone()))?;
            return Ok(Oc::Ocg(ocg));
        }

        let ocgs = read_optional_field(doc, dict);
        let policy = match read_optional_field(doc, dict) {
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
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
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
}
