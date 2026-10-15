//! Optional Content Config
//!
//! ISO 32000-1:2008 8.11.4.3 Optional Content Configuration Dictionaries

use crate::codec::{TryFromObject, TryIntoObject};
use crate::dict::{self, DictKey, read_optional_field};
use crate::error::{OptionalField, OptionalFieldExt, ResultExt};
use crate::optional_content::{Oc, Ocg, Ocgs, OcmdPolicy};
use crate::{Error, Pdf, Result};
use lopdf::{Dictionary, Object, ObjectId, decode_text_string, text_string};

const D_ORDER_DEPTH_LIMIT: usize = 128;

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
    name: OptionalField<Name>,
    creator: OptionalField<Creator>,
    pub(crate) base_state: Result<BaseState>,
    pub(crate) on: Result<DOn>,
    pub(crate) off: Result<DOff>,
    pub(crate) order: Result<DOrder>,
}

impl PartialEq for OcConfig {
    fn eq(&self, other: &Self) -> bool {
        let (Ok(base_state), Ok(on), Ok(off), Ok(order)) = (
            self.base_state.ok_ref(),
            self.on.ok_ref(),
            self.off.ok_ref(),
            self.order.ok_ref(),
        ) else {
            return false;
        };
        let (Ok(other_base_state), Ok(other_on), Ok(other_off), Ok(other_order)) = (
            other.base_state.ok_ref(),
            other.on.ok_ref(),
            other.off.ok_ref(),
            other.order.ok_ref(),
        ) else {
            return false;
        };
        let name_eq = match (&self.name, &other.name) {
            (None, None) => true,
            (Some(Ok(self_name)), Some(Ok(other_name))) => self_name == other_name,
            (_, _) => false,
        };
        let creator_eq = match (&self.creator, &other.creator) {
            (None, None) => true,
            (Some(Ok(self_creator)), Some(Ok(other_creator))) => self_creator == other_creator,
            (_, _) => false,
        };
        name_eq
            && creator_eq
            && base_state == other_base_state
            && on == other_on
            && off == other_off
            && order == other_order
    }
}

impl DictKey for OcConfig {
    const KEY: &'static [u8] = b"D";
}

impl Default for OcConfig {
    fn default() -> Self {
        Self {
            name: None,
            creator: None,
            base_state: Ok(Default::default()),
            on: Ok(Default::default()),
            off: Ok(Default::default()),
            order: Ok(Default::default()),
        }
    }
}

impl OcConfig {
    /// Create an [`OcConfig`] by providing the base state, on / off array and the order.
    pub fn new(
        name: Option<Name>,
        base_state: BaseState,
        on: DOn,
        off: DOff,
        order: DOrder,
    ) -> Self {
        Self {
            name: name.map(Ok),
            creator: Some(Ok(Creator::trykklar())),
            base_state: Ok(base_state),
            on: Ok(on),
            off: Ok(off),
            order: Ok(order),
        }
    }

    /// Creates an [`OcConfig`] with the given on entry, basestate set to [`BaseState::Off`] and the
    /// default values for the rest.
    pub fn new_with_on(on: DOn) -> Self {
        Self::new(None, BaseState::Off, on, DOff::default(), DOrder::default())
    }

    /// Returns the [`Creator`].
    pub fn creator(&self) -> OptionalField<&Creator> {
        self.creator.as_field_ref()
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
    fn try_from_object(pdf: &'a Pdf, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let Object::Dictionary(dict) = obj else {
            return Err(Error::InvalidPdfObject(
                "configuration must be a dictionary",
            ));
        };
        let name = read_optional_field(pdf, dict);
        let creator = read_optional_field(pdf, dict);
        let base_state = match read_optional_field(pdf, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        let on = match read_optional_field(pdf, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        let off = match read_optional_field(pdf, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        let order = match read_optional_field(pdf, dict) {
            Some(bs) => bs,
            None => Ok(Default::default()),
        };
        Ok(Self {
            name,
            creator,
            base_state,
            on,
            off,
            order,
        })
    }
}

impl TryIntoObject for OcConfig {
    fn try_into_object(self, pdf: &mut Pdf) -> Result<Object> {
        let default_config_id = pdf.doc_mut().add_object(Dictionary::new());
        dict::write_optional_field(self.name, pdf, |pdf: &mut Pdf| {
            dict::get_mut(default_config_id, pdf)
        })?;
        dict::write_optional_field(self.creator, pdf, |pdf: &mut Pdf| {
            dict::get_mut(default_config_id, pdf)
        })?;
        dict::write(self.base_state?, pdf, |pdf: &mut Pdf| {
            dict::get_mut(default_config_id, pdf)
        })?;
        dict::write(self.on?, pdf, |pdf: &mut Pdf| {
            dict::get_mut(default_config_id, pdf)
        })?;
        dict::write(self.off?, pdf, |pdf: &mut Pdf| {
            dict::get_mut(default_config_id, pdf)
        })?;
        dict::write(self.order?, pdf, |pdf: &mut Pdf| {
            dict::get_mut(default_config_id, pdf)
        })?;
        Ok(Object::Reference(default_config_id))
    }
}

/// `/Name`
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) A name for the configuration, suitable for presentation in a user interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(String);

impl Name {
    /// Creates a name.
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Returns the creator.
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl DictKey for Name {
    const KEY: &'static [u8] = b"Name";
}

impl TryFromObject<'_> for Name {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let name = decode_text_string(obj)?;
        Ok(Self(name))
    }
}

impl TryIntoObject for Name {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/Creator`
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) Name of the application or feature that created this configuration dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Creator(String);

impl Creator {
    fn trykklar() -> Self {
        Self(String::from("trykklar"))
    }

    /// Returns the creator.
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl DictKey for Creator {
    const KEY: &'static [u8] = b"Creator";
}

impl TryFromObject<'_> for Creator {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let creator = decode_text_string(obj)?;
        Ok(Self(creator))
    }
}

impl TryIntoObject for Creator {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}
/// `/ON` Array of OCG with state on
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) An array of optional content groups whose state shall be set to ON when this
/// > configuration is applied. If the BaseState entry is ON, this entry is redundant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DOn(Vec<Ocg>);

impl DOn {
    /// Returns the set of [`Ocg`].
    pub fn get(&self) -> &[Ocg] {
        &self.0
    }

    /// Creates an ON object.
    pub fn new(ocgs: Vec<Ocg>) -> Self {
        Self(ocgs)
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
    fn try_from_object(pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let Object::Array(arr) = obj else {
            return Err(Error::InvalidPdfObject("D On must be an array"));
        };
        let mut ocgs = Vec::new();
        for element_obj in arr {
            let (ocg_id, ocg_obj) = pdf.doc().dereference(element_obj)?;
            let ocg = Ocg::try_from_object(pdf, ocg_id, ocg_obj)?;
            ocgs.push(ocg);
        }
        Ok(Self(ocgs))
    }
}

impl TryIntoObject for DOn {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        let ocgs = self
            .0
            .into_iter()
            .map(|ocg| Object::Reference(ocg.id.into()))
            .collect();
        Ok(Object::Array(ocgs))
    }
}

/// `/OFF` Array of OCG with state off
///
/// ISO 32000-1:2008 8.11.4.3 Table 101 – Entries in an Optional Content Configuration Dictionary
///
/// > (Optional) An array of optional content groups whose state shall be set to OFF when this
/// > configuration is applied. If the BaseState entry is OFF, this entry is redundant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DOff(Vec<Ocg>);

impl DOff {
    /// Returns the set of [`Ocg`].
    pub fn get(&self) -> &[Ocg] {
        &self.0
    }

    /// Creates an ON object.
    pub fn new(ocgs: Vec<Ocg>) -> Self {
        Self(ocgs)
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
    fn try_from_object(pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let Object::Array(arr) = obj else {
            return Err(Error::InvalidPdfObject("D Off must be an array"));
        };
        let mut ocgs = Vec::new();
        for element_obj in arr {
            let (ocg_id, ocg_obj) = pdf.doc().dereference(element_obj)?;
            let ocg = Ocg::try_from_object(pdf, ocg_id, ocg_obj)?;
            ocgs.push(ocg);
        }
        Ok(Self(ocgs))
    }
}

impl TryIntoObject for DOff {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        let ocgs = self
            .0
            .into_iter()
            .map(|ocg| Object::Reference(ocg.id.into()))
            .collect();
        Ok(Object::Array(ocgs))
    }
}

/// An optionally named group of [`DOrderItem`].
#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn into_array_of_objects(self, pdf: &mut Pdf) -> Result<Vec<Object>> {
        let mut arr = Vec::new();
        if let Some(name) = &self.name {
            arr.push(text_string(name));
        }
        let items_arr = d_order_items_to_object(self.items, pdf)?;
        arr.extend(items_arr);
        Ok(arr)
    }
}

/// A group of OCGs with an Ocg as header.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn into_array_of_objects(self, pdf: &mut Pdf) -> Result<Vec<Object>> {
        let mut arr = Vec::new();
        arr.push(self.header.try_into_object(pdf)?);
        let body_items_arr = d_order_items_to_object(self.body, pdf)?;
        arr.push(Object::Array(body_items_arr));
        Ok(arr)
    }
}

/// Find the definition in the parent object [`DOrder`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DOrderItem {
    /// Optional content group [`Ocg`].
    Ocg(Ocg),
    /// Array of an optional content group [`Ocg`] with optional header name.
    OcgGroup(OcgGroup),
    /// Array of an optional content group [`Ocg`] with an Ocg as header.
    OcgSubGroup(OcgSubGroup),
}

impl From<Ocg> for DOrderItem {
    fn from(value: Ocg) -> Self {
        Self::Ocg(value)
    }
}

impl From<OcgGroup> for DOrderItem {
    fn from(value: OcgGroup) -> Self {
        Self::OcgGroup(value)
    }
}

impl From<OcgSubGroup> for DOrderItem {
    fn from(value: OcgSubGroup) -> Self {
        Self::OcgSubGroup(value)
    }
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DOrder(Vec<DOrderItem>);

impl DOrder {
    /// Returns the array of [`DOrderItem`].
    pub fn get(&self) -> &[DOrderItem] {
        &self.0
    }
}

impl FromIterator<DOrderItem> for DOrder {
    fn from_iter<I: IntoIterator<Item = DOrderItem>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl DictKey for DOrder {
    const KEY: &'static [u8] = b"Order";
}

/// This is the recursive resolving of arrays of DOrderItem as defined in [`DOrder`].
fn d_order_from_array<'a>(
    pdf: &'a Pdf,
    objects: &'a [Object],
    depth: usize,
) -> Result<Vec<DOrderItem>> {
    if depth > D_ORDER_DEPTH_LIMIT {
        return Err(Error::OcPropertiesNotFound);
    }

    let mut items = Vec::new();

    let mut buffer_ocg: Option<Ocg> = None;
    let mut buffer_group: Option<OcgGroup> = None;
    let mut buffer_title: Option<String> = None;

    let mut obj_iter = objects.iter();
    let mut idx = 0;
    while let Some(obj) = obj_iter.next()
        && let (id, obj) = pdf.doc().dereference(obj)?
    {
        if let Ok(title) = decode_text_string(obj) {
            if idx > 0 {
                return Err(Error::InvalidPdfObject(
                    "only the first entry can be a name entry",
                ));
            }
            buffer_title = Some(title);
        } else {
            match (
                id,
                obj,
                buffer_title.take(),
                buffer_group.take(),
                buffer_ocg.take(),
            ) {
                (_, Object::Array(arr), None, None, Some(ocg)) => {
                    let arr_items = d_order_from_array(pdf, arr, depth + 1)?;
                    if arr.len() > 1 && arr_items.len() == 1 {
                        items.push(ocg.into());
                        items.extend(arr_items);
                    } else {
                        items.push(
                            OcgSubGroup {
                                header: ocg,
                                body: arr_items,
                            }
                            .into(),
                        )
                    }
                }
                (_, Object::Array(arr), None, None, None) => {
                    let arr_items = d_order_from_array(pdf, arr, depth + 1)?;
                    if matches!(arr_items.first(), Some(DOrderItem::OcgGroup(..))) {
                        items.extend(arr_items);
                    } else {
                        items.push(
                            OcgGroup {
                                name: None,
                                items: arr_items,
                            }
                            .into(),
                        );
                    }
                }
                (id, val, None, None, Some(ocg)) => {
                    items.push(ocg.into());
                    let ocg = Ocg::try_from_object(pdf, id, val)?;
                    buffer_ocg = Some(ocg);
                }
                (id, val, None, None, None) => {
                    let ocg = Ocg::try_from_object(pdf, id, val)?;
                    buffer_ocg = Some(ocg);
                }
                (id, val, None, Some(mut group), None) => {
                    let ocg = Ocg::try_from_object(pdf, id, val)?;
                    group.items.push(ocg.into());
                    buffer_group = Some(group);
                }
                (id, val, Some(title), None, None) => {
                    let ocg = Ocg::try_from_object(pdf, id, val)?;
                    buffer_group = Some(OcgGroup {
                        name: Some(title),
                        items: vec![ocg.into()],
                    });
                }
                _ => return Err(Error::InvalidPdfObject("d order could not be parsed")),
            }
        }
        idx += 1;
    }
    if let Some(ocg) = buffer_ocg.take() {
        items.push(ocg.into());
    }
    if let Some(group) = buffer_group.take() {
        items.push(group.into());
    }

    Ok(items)
}

impl<'a> TryFromObject<'a> for DOrder {
    fn try_from_object(pdf: &'a Pdf, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        match obj {
            Object::Array(array) => Ok(Self(d_order_from_array(pdf, array, 0)?)),
            _ => Err(Error::InvalidPdfObject("DOrder must be an array")),
        }
    }
}

fn d_order_items_to_object(items: Vec<DOrderItem>, pdf: &mut Pdf) -> Result<Vec<Object>> {
    let mut arr = Vec::new();
    for item in items.into_iter() {
        match item {
            DOrderItem::Ocg(ocg) => arr.push(ocg.try_into_object(pdf)?),
            DOrderItem::OcgSubGroup(sub_group) => arr.extend(sub_group.into_array_of_objects(pdf)?),
            DOrderItem::OcgGroup(group) => {
                arr.push(Object::Array(group.into_array_of_objects(pdf)?))
            }
        }
    }
    Ok(arr)
}

impl TryIntoObject for DOrder {
    fn try_into_object(self, pdf: &mut Pdf) -> Result<Object> {
        Ok(Object::Array(d_order_items_to_object(self.0, pdf)?))
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
    fn try_from_object(_pdf: &'a Pdf, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
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

impl TryIntoObject for BaseState {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        let value = match self {
            Self::On => "ON",
            Self::Off => "OFF",
            Self::Unchanged => "Unchanged",
        };
        Ok(Object::Name(value.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optional_content::OCPropertiesMut;

    #[test]
    fn compare_oc_config() -> Result<()> {
        assert_eq!(OcConfig::default(), OcConfig::default());
        let oc_config_a = OcConfig::new(
            None,
            BaseState::On,
            DOn::default(),
            DOff::default(),
            DOrder::default(),
        );
        let oc_configs_b = OcConfig::new(
            None,
            BaseState::Off,
            DOn::default(),
            DOff::default(),
            DOrder::default(),
        );
        assert_ne!(oc_config_a, oc_configs_b);
        Ok(())
    }

    #[test]
    fn replace_oc_properties_default_config() -> Result<()> {
        let pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let default_config_before = pdf.catalog()?.oc_properties().unwrap()?.default_config()?;
        let mut pdf = pdf;
        let mut oc_props_mut = OCPropertiesMut::try_from(&mut pdf)?;
        oc_props_mut.set_default_config(default_config_before.clone())?;
        let default_config_after = pdf.catalog()?.oc_properties().unwrap()?.default_config()?;
        assert_eq!(default_config_before, default_config_after);
        let oc_config = OcConfig::default();
        let mut oc_props_mut = OCPropertiesMut::try_from(&mut pdf)?;
        oc_props_mut.set_default_config(oc_config.clone())?;
        let config = pdf.catalog()?.oc_properties().unwrap()?.default_config()?;
        assert_eq!(oc_config, config);
        Ok(())
    }

    #[test]
    fn default_config_encode_decode() -> Result<()> {
        let mut pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let ocgs = pdf
            .catalog()?
            .oc_properties()
            .expect("exists")?
            .ocgs()?
            .get()
            .to_owned();
        let order = DOrder::from_iter(vec![
            OcgSubGroup {
                header: ocgs[1].clone(),
                body: vec![ocgs[1].clone().into(), ocgs[1].clone().into()],
            }
            .into(),
            OcgSubGroup {
                header: ocgs[1].clone(),
                body: vec![ocgs[1].clone().into()],
            }
            .into(),
            OcgGroup {
                name: Some("Group title".to_string()),
                items: vec![ocgs[1].clone().into(), ocgs[1].clone().into()],
            }
            .into(),
            OcgGroup {
                name: None,
                items: vec![ocgs[1].clone().into(), ocgs[1].clone().into()],
            }
            .into(),
            ocgs[1].clone().into(),
            ocgs[1].clone().into(),
            OcgGroup {
                name: Some("Group title".to_string()),
                items: vec![ocgs[1].clone().into()],
            }
            .into(),
            OcgSubGroup {
                header: ocgs[1].clone(),
                body: vec![ocgs[1].clone().into()],
            }
            .into(),
            OcgGroup {
                name: None,
                items: vec![ocgs[1].clone().into(), ocgs[1].clone().into()],
            }
            .into(),
        ]);
        let oc_config = OcConfig::new(
            None,
            BaseState::default(),
            DOn::default(),
            DOff::default(),
            order,
        );
        let mut oc_props_mut = OCPropertiesMut::try_from(&mut pdf)?;
        oc_props_mut.set_default_config(oc_config.clone())?;
        let pdf_oc_config = pdf
            .catalog()?
            .oc_properties()
            .expect("exists")?
            .default_config()?;
        assert_eq!(pdf_oc_config.order()?, oc_config.order()?);
        Ok(())
    }

    #[test]
    fn idempotent_d_on_off_encode_decode() -> Result<()> {
        let mut pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let ocgs = pdf
            .catalog()?
            .oc_properties()
            .expect("exists")?
            .ocgs()?
            .get()
            .to_owned();
        let oc_config = OcConfig::new(
            Some(Name::new("OC Config")),
            BaseState::default(),
            DOn::from_iter(ocgs[..2].to_vec()),
            DOff::from_iter(ocgs[2..].to_vec()),
            DOrder::default(),
        );
        let mut oc_props_mut = OCPropertiesMut::try_from(&mut pdf)?;
        oc_props_mut.set_default_config(oc_config.clone())?;
        let pdf_oc_config = pdf
            .catalog()?
            .oc_properties()
            .expect("exists")?
            .default_config()?;
        assert_eq!(pdf_oc_config, oc_config);
        Ok(())
    }
}
