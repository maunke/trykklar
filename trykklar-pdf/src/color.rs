//! Color Module
use crate::codec::{TryFromObject, deref_array, deref_name};
use crate::content::ResolvedCache;
use crate::dict::{DictKey, read_field};
use crate::error::{Field, FieldExt, ResultExt};
use crate::{Error, ImageId, Result, ShadingPattern, TilingPattern};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::hash::Hash;
use std::sync::Arc;

/// Color Pattern
///
/// This enum is used to cover the `/SCN` and `/scn` operator spec (see ISO 32000-1:2008 Table 74 -
/// Colour Operators).
#[derive(Debug, Clone)]
pub enum PatternColor<'a> {
    /// Color componens set for an uncolored tiling pattern
    ///
    /// This variant originates from [`Color::Pattern`]. Read the introduction there.
    ///
    /// It relates to the uncoloured tiling pattern (ISO 32000-1:2008), meaning the the tiling
    /// pattern type is [`crate::pattern::TilingPaintType::Uncoloured`].
    UncoloredTiling {
        /// Tiling Pattern
        pattern: Arc<TilingPattern<'a>>,
        /// Values containing color components
        values: Box<[f32]>,
    },
    /// Colored tiling pattern with [`crate::pattern::TilingPaintType::Coloured`].
    ColoredTiling(Arc<TilingPattern<'a>>),
    /// Shading pattern
    Shading(Arc<ShadingPattern>),
}

/// Color
///
/// This object covers two arms of colour-setting operators as defined in ISO 32000-1:2008 Table 74:
///
/// - Color Values as introduced in 8.6.2 Colour Values
/// - Color Pattern, covering the `/SCN` and `/scn` operator spec
///
/// Color values are loosely coupled, that they are not restricted to match the number of components
/// of the resent color space. This is part of the lenient parsing strategy.
#[derive(Debug, Clone)]
pub enum Color<'a> {
    /// Values, consisting of color components
    Values(Box<[f32]>),
    /// Pattern, covering the `/SCN` and `/scn` operator spec
    ///
    /// This contains the [`PatternColor::UncoloredTiling`] variant.
    ///
    /// Since [`crate::TilingPattern`] is used in the [`crate::ContentWalker`], in order to start a
    /// new walker from the tiling pattern, the PDF spec tiling pattern (ISO 32000-1:2008 8.7
    /// Patterns).
    Pattern(PatternColor<'a>),
}

/// Colour Space Variants
///
/// ISO 32000-1:2008 8.6.3 Coluur Space Families
///
/// > Colour spaces are classified into colour space families. Spaces within a family share the same
/// > general characteristics; they shall be distinguished by parameter values supplied at the time
/// > the space is specified. The families fall into three broad categories:
///
/// These are device, CIE-based and special color spaces.
///
/// ISO 32000-1:2008 8.6.3 Table 62 - Colour Space Families:
///
/// - Device: [`ColorSpace::DeviceGray`], [`ColorSpace::DeviceRgb`], [`ColorSpace::DeviceCmyk`]
/// - CIE-based: [`ColorSpace::CalGray`], [`ColorSpace::CalRgb`], [`ColorSpace::Lab`],
///   [`ColorSpace::IccBased`]
/// - Special: [`ColorSpace::Indexed`], [`ColorSpace::Pattern`], [`ColorSpace::Separation`],
///   [`ColorSpace::DeviceN`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    // Device Family
    /// DeviceGray (PDF 1.1)
    DeviceGray,
    /// DeviceRGB (PDF 1.1)
    DeviceRgb,
    /// DeviceCMYK (PDF 1.1)
    DeviceCmyk,
    // CIE-based Family
    /// CalGray (PDF 1.1)
    CalGray,
    /// CalRGB (PDF 1.1)
    CalRgb,
    /// Lab (PDF 1.1)
    Lab,
    /// ICCBased (PDF 1.3)
    IccBased(Arc<IccBased>),
    // Special Family
    /// Indexed (PDF 1.1)
    Indexed(Arc<Indexed>),
    /// Pattern (PDF 1.2)
    Pattern(Option<Arc<ColorSpace>>),
    /// Separation (PDF 1.2)
    Separation(Arc<Separation>),
    /// DeviceN (PDF 1.3)
    DeviceN(Arc<DeviceN>),
}

impl DictKey for ColorSpace {
    const KEY: &'static [u8] = b"ColorSpace";
}

impl ColorSpace {
    /// Returns the default color for the hereby colorspace.
    ///
    /// The definition originates from ISO 32000-1:2008 Table 74 for the CS operator and its initial
    /// value:
    ///
    /// > In a DeviceGray, DeviceRGB, CalGray, or CalRGB colour space, the initial colour shall
    /// > have all components equal to 0.0.
    /// >
    /// > In a DeviceCMYK colour space, the initial colour shall be [ 0.0 0.0 0.0 1.0 ].
    /// >
    /// > In a Lab or ICCBased colour space, the initial colour shall have all components equal to
    /// > 0.0 unless that falls outside the intervals specified by the space’s Range entry, in which
    /// > case the nearest valid value shall be substituted.
    /// >
    /// > In an Indexed colour space, the initial colour value shall be 0.
    ///
    /// The following color spaces do not play a role for the return type of [`Color`] since they
    /// more relates to rendering.
    ///
    /// > In a Separation or DeviceN colour space, the initial tint value shall be 1.0 for all
    /// > colorants.
    /// >
    /// > In a Pattern colour space, the initial colour shall be a pattern object that causes
    /// > nothing to be painted.
    pub fn default_color(&self) -> Color<'static> {
        Color::Values(match self {
            ColorSpace::DeviceGray | ColorSpace::CalGray => vec![0.0].into_boxed_slice(),
            ColorSpace::DeviceRgb | ColorSpace::CalRgb | ColorSpace::Lab => {
                vec![0.0, 0.0, 0.0].into_boxed_slice()
            }
            ColorSpace::DeviceCmyk => vec![0.0, 0.0, 0.0, 1.0].into_boxed_slice(),
            _ => vec![].into_boxed_slice(),
        })
    }
}

/// DeviceN
///
/// ISO 32000-1:2008 8.6.6.5 DeviceN Colour Spaces
///
/// > DeviceN colour spaces shall be defined in a similar way to Separation colour spaces—in fact, a
/// > Separation colour space can be defined as a DeviceN colour space with only one component.
/// >
/// > A DeviceN colour space shall be specified as follows:
/// >
/// > `[ /DeviceN names alternateSpace tintTransform ]`
/// >
/// > or
/// >
/// > `[ /DeviceN names alternateSpace tintTransform attributes ]`
/// >
/// > It is a four- or five-element array whose first element shall be the colour space family name
/// > DeviceN. The remaining elements shall be parameters that a DeviceN colour space requires.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceN {
    names: Vec<SeparationName>,
    alternate: ColorSpace,
}

impl DeviceN {
    /// Creates a new DeviceN color space, requiring names vec to be not empty.
    pub fn try_new(names: Vec<SeparationName>, alternate: ColorSpace) -> Result<Self> {
        if names.is_empty() {
            return Err(Error::InvalidColorSpace);
        }
        Ok(Self { names, alternate })
    }

    /// Returns the names slice.
    pub fn names(&self) -> &[SeparationName] {
        &self.names
    }

    /// Returns the alternate [`ColorSpace`].
    pub fn alternate(&self) -> &ColorSpace {
        &self.alternate
    }
}

/// Icc Profile ID
///
/// [Specification ICC.1:2022 (Profile version 4.4.0.0)](https://archive.color.org/specification/ICC.1-2022-05.pdf) 7.2.18 Profile ID field (bytes 84 to 99):
///
/// > This field, if not zero (00h), shall hold the Profile ID. The Profile ID shall be calculated
/// > using the MD5 fingerprinting method as defined in Internet RFC 1321. The entire profile, whose
/// > length is given by the size field in the header, with the profile flags field (bytes 44 to 47,
/// > see 7.2.11), rendering intent field (bytes 64 to 67, see  ICC.1:2022 24 © ICC 2022 – All
/// > rights reserved 7.2.15), and profile ID field (bytes 84 to 99) in the profile header
/// > temporarily set to zeros (00h), shall be used to calculate the ID. A profile ID field value of
/// > zero (00h) shall indicate that a profile ID has not been calculated.
/// >
/// > Profile creators should compute and record a profile ID.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub struct IccProfileId([u8; 16]);

impl IccProfileId {
    /// Returns the 16 bytes profile id.
    pub fn get(&self) -> [u8; 16] {
        self.0
    }

    // Returns the found or computed Profile ID
    //
    // Read the definition on [`IccProfileId`].
    fn from_profile(content: &[u8]) -> Result<IccProfileId> {
        let embedded: [u8; 16] = content
            .get(84..100)
            .and_then(|s| s.try_into().ok())
            .ok_or(Error::InvalidIccProfile)?;
        if embedded != [0u8; 16] {
            return Ok(IccProfileId(embedded));
        }
        let mut buf = content.to_vec();
        // zero flags and rendering intent
        for range in [44..48, 64..68] {
            buf.get_mut(range).ok_or(Error::InvalidIccProfile)?.fill(0);
        }
        Ok(IccProfileId(md5::compute(&buf).into()))
    }
}

/// Object ID in the document.
///
/// The ICCBased colorspace can have two origins: a referenced object or defined by a Jpx image
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum IccBasedId {
    /// Document Object Id
    Object(ObjectId),
    /// Image ID of the Jpx image containing the profile
    Image(ImageId),
}

/// Icc Profile Description
///
/// [Specification ICC.1:2022 (Profile version 4.4.0.0)](https://archive.color.org/specification/ICC.1-2022-05.pdf) 9.2.43 profileDescriptionTag
#[derive(Debug, Clone)]
pub struct IccProfileDescription(pub(crate) String);

impl IccProfileDescription {
    /// Returns the description string.
    pub fn get(&self) -> &str {
        &self.0
    }

    // See Profile Description Tag definition in the ICC spec.
    // v2: textDescriptionType
    // v4: multiLocalizeUnicodeType
    fn from_profile(content: &[u8]) -> Option<Self> {
        let tag_count = u32::from_be_bytes(content.get(128..132)?.try_into().ok()?) as usize;
        let table = content.get(132..132 + tag_count.checked_mul(12)?)?;
        let entry = table.chunks_exact(12).find(|e| &e[0..4] == b"desc")?;
        let offset = u32::from_be_bytes(entry[4..8].try_into().ok()?) as usize;
        let size = u32::from_be_bytes(entry[8..12].try_into().ok()?) as usize;
        let tag = content.get(offset..offset.checked_add(size)?)?;

        match tag.get(0..4)? {
            // v2: [sig4][reserved4][count u32][ascii..]
            b"desc" => {
                let count = u32::from_be_bytes(tag.get(8..12)?.try_into().ok()?) as usize;
                let ascii = tag.get(12..12usize.checked_add(count)?)?;
                let end = ascii.iter().position(|&b| b == 0).unwrap_or(ascii.len());
                Some(Self(String::from_utf8_lossy(&ascii[..end]).into_owned()))
            }
            // v4: [sig4][reserved4][num u32][recsize u32][records..]
            // record: [lang2][country2][len u32][offset u32]
            b"mluc" => {
                if u32::from_be_bytes(tag.get(8..12)?.try_into().ok()?) == 0 {
                    return None;
                }
                let rec = tag.get(16..28)?;
                let len = u32::from_be_bytes(rec[4..8].try_into().ok()?) as usize;
                let off = u32::from_be_bytes(rec[8..12].try_into().ok()?) as usize;
                let utf16 = tag.get(off..off.checked_add(len)?)?;
                let units: Vec<u16> = utf16
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                Some(Self(String::from_utf16_lossy(&units)))
            }
            _ => None,
        }
    }
}

/// ICCBased ColorSpace
///
/// ISO 32000-1:2008 8.6.5.5 ICCBased Colour Spaces
/// Table 66 - Additional Entries Specific to an ICC Profile Stream Dictionary
#[derive(Debug, Clone)]
pub struct IccBased {
    id: IccBasedId,
    n: Field<IccComponents>,
    profile_id: Result<IccProfileId>,
    description: Option<IccProfileDescription>,
}

impl PartialEq for IccBased {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for IccBased {}
impl Hash for IccBased {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl IccBased {
    fn resolve(stream_obj: &Object, doc: &Document) -> Result<Self> {
        let stream_obj = doc.dereference(stream_obj)?;
        let stream_id = stream_obj.0.ok_or(Error::InvalidPdfObject(
            "ICCBased profile must be an indirect stream",
        ))?;
        let id = IccBasedId::Object(stream_id);
        let stream = stream_obj.1.as_stream()?;
        let dict = &stream.dict;
        let n = read_field(doc, dict);
        let content = stream.get_plain_content()?;
        let profile_id = IccProfileId::from_profile(&content);
        let description = IccProfileDescription::from_profile(&content);
        Ok(Self {
            id,
            n,
            profile_id,
            description,
        })
    }

    pub(crate) fn try_from_jpx(
        image_id: ImageId,
        content: &[u8],
        num_channels: u8,
    ) -> Result<Self> {
        let id = IccBasedId::Image(image_id);
        let n = Ok(IccComponents::try_from(num_channels as i64)?);
        let profile_id = IccProfileId::from_profile(content);
        let description = IccProfileDescription::from_profile(content);
        Ok(Self {
            id,
            n,
            profile_id,
            description,
        })
    }

    /// Returns the number of color components.
    pub fn n(&self) -> Field<&IccComponents> {
        self.n.as_field_ref()
    }

    /// Returns the profile ID.
    pub fn profile_id(&self) -> Result<IccProfileId> {
        self.profile_id.ok_ref().copied()
    }

    /// Returns the profile description.
    pub fn description(&self) -> Option<&IccProfileDescription> {
        self.description.as_ref()
    }
}

/// `/N` The number of colour components in ICC profile
///
/// ISO 32000-1:2008 Table 66 – Additional Entries Specific to an ICC Profile Stream Dictionary
///
/// > (Required) The number of colour components in the colour space described by the ICC profile
/// > data. This number shall match the number of components actually in the ICC profile. N shall be
/// > 1, 3, or 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IccComponents {
    /// `1`
    One,
    /// `3`
    Three,
    /// `4`
    Four,
}

impl DictKey for IccComponents {
    const KEY: &'static [u8] = b"N";
}

impl TryFromObject<'_> for IccComponents {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        Self::try_from(obj.as_i64()?)
    }
}

impl TryFrom<i64> for IccComponents {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        let comp = match value {
            1 => Self::One,
            3 => Self::Three,
            4 => Self::Four,
            _ => {
                return Err(Error::InvalidPdfObject(
                    "IccBased N must be one of 1, 3 or 4",
                ));
            }
        };
        Ok(comp)
    }
}

/// Separation Color Space
///
/// ISO 3200-1:2008 8.6.6.4 Separation Colour Spaces
///
/// > A Separation colour space (PDF 1.2) provides a means for specifying the use of additional
/// > colorants or for isolating the control of individual colour components of a device colour
/// > space for a subtractive device. When such a space is the current colour space, the current
/// > colour shall be a single-component value, called a tint, that controls the application of the
/// > given colorant or colour components only.
/// >
/// > A Separation colour space is defined as follows:
/// >
/// > `[ /Separation name alternateSpace tintTransform ]`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Separation {
    pub(crate) name: SeparationName,
    pub(crate) alternate: ColorSpace,
}

impl Separation {
    /// Returns the name of the separation.
    pub fn name(&self) -> &SeparationName {
        &self.name
    }

    /// Returns the alternate color space.
    pub fn alternate(&self) -> &ColorSpace {
        &self.alternate
    }
}

/// Separation Name
///
/// ISO 3200-1:2008 8.6.6.4 Separation Colour Spaces
///
/// > The name parameter is a name object that shall specify the name of the colorant that this
/// > Separation colour space is intended to represent (or one of the special names All or None; see
/// > below). Such colorant names are arbitrary, and there may be any number of them, subject to
/// > implementation limits.
/// >
/// > [All and None member definition ...]
/// >
/// > A conforming reader shall support Separation colour spaces with the colorant names All and
/// > None on all devices, even if the devices are not capable of supporting any others. When
/// > processing Separation spaces with either of these colorant names conforming readers shall
/// > ignore the alternateSpace and tintTransform parameters (discussed below), although valid
/// > values shall still be provided.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SeparationName {
    /// > The special colorant name All shall refer collectively to all colorants available on an
    /// > output device, including those for the standard process colorants. When a Separation space
    /// > with this colorant name is the current colour space, painting operators shall apply tint
    /// > values to all available colorants at once.
    All,
    /// > The special colorant name None shall not produce any visible output. Painting operations
    /// > in a Separation space with this colorant name shall have no effect on the current page.
    None,
    /// Separation name
    Name(String),
}

/// Indexed Color Space
///
/// ISO 32000-1:2008 8.6.6.3 Indexed Colour Spaces
///
/// > An Indexed colour space specifies that an area is to be painted using a colour map or colour
/// > table of arbitrary colours in some other space. A conforming reader shall treat each sample
/// > value as an index into the colour table and shall use the colour value it finds there. This
/// > technique can considerably reduce the amount of data required to represent a sampled image.
/// >
/// > An Indexed colour space shall be defined by a four-element array:
/// >
/// > `[ /Indexed base hival lookup ]`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Indexed {
    base: ColorSpace,
}

impl Indexed {
    /// Returns the base color space.
    pub fn base(&self) -> &ColorSpace {
        &self.base
    }
}

impl ColorSpace {
    pub(crate) fn resolve(
        value: &lopdf::content::Operation,
        resource_dicts: &[&Dictionary],
        doc: &Document,
        cache: &mut ResolvedCache<Self>,
    ) -> Result<Self> {
        let lopdf::content::Operation { operator, operands } = value;
        let color_space = match operator.as_str() {
            "CS" | "cs" => Self::parse_cs(operands, resource_dicts, doc, cache)?,
            "G" | "g" => Self::DeviceGray,
            "RG" | "rg" => Self::DeviceRgb,
            "K" | "k" => Self::DeviceCmyk,
            _ => return Err(Error::InvalidOperator),
        };
        Ok(color_space)
    }

    pub(crate) fn parse_cs(
        operands: &[Object],
        resource_dicts: &[&Dictionary],
        doc: &Document,
        cache: &mut ResolvedCache<Self>,
    ) -> Result<Self> {
        let [Object::Name(name)] = operands else {
            return Err(Error::InvalidOperands);
        };
        cache.get_or_resolve(name, || Self::parse_name(name, resource_dicts, doc, 0))
    }

    const MAX_CYCLES: usize = 128;

    fn parse_name(
        name: &[u8],
        resource_dicts: &[&Dictionary],
        doc: &Document,
        cycle_count: usize,
    ) -> Result<Self> {
        let cycle_count = cycle_count + 1;
        if cycle_count >= Self::MAX_CYCLES {
            return Err(Error::InvalidColorSpace);
        }
        let cs = match name {
            b"DeviceGray" => Self::DeviceGray,
            b"DeviceRGB" => Self::DeviceRgb,
            b"DeviceCMYK" => Self::DeviceCmyk,
            b"Pattern" => Self::Pattern(None),
            key_name => {
                for d in resource_dicts {
                    if let Ok((_, Object::Dictionary(cs_dict))) =
                        d.get(b"ColorSpace").and_then(|o| doc.dereference(o))
                        && let Ok(cs_obj) = cs_dict.get(key_name)
                    {
                        return Self::parse_object(cs_obj, resource_dicts, doc, cycle_count);
                    }
                }
                return Err(Error::UndefinedColorSpace);
            }
        };
        Ok(cs)
    }

    pub(crate) fn parse_object(
        obj: &Object,
        resource_dicts: &[&Dictionary],
        doc: &Document,
        cycle_count: usize,
    ) -> Result<Self> {
        let cycle_count = cycle_count + 1;
        if cycle_count >= Self::MAX_CYCLES {
            return Err(Error::InvalidColorSpace);
        }
        let (_, obj) = doc.dereference(obj)?;
        match obj {
            Object::Name(name) => Self::parse_name(name, resource_dicts, doc, cycle_count),
            Object::Array(arr) => {
                let Some(Object::Name(cs_name)) = arr.first() else {
                    return Err(Error::InvalidColorSpace);
                };
                let cs = match cs_name.as_slice() {
                    b"CalGray" => Self::CalGray,
                    b"CalRGB" => Self::CalRgb,
                    b"Lab" => Self::Lab,
                    b"ICCBased" => {
                        let Some(stream_obj) = arr.get(1) else {
                            return Err(Error::InvalidOperands);
                        };
                        Self::IccBased(Arc::new(IccBased::resolve(stream_obj, doc)?))
                    }
                    b"Indexed" => {
                        let [_, ref base_obj, ref _hival, ref _lookup] = arr[..] else {
                            return Err(Error::InvalidColorSpace);
                        };
                        let base = Self::parse_object(base_obj, resource_dicts, doc, cycle_count)?;
                        Self::Indexed(Indexed { base }.into())
                    }
                    b"Pattern" => match arr.get(1) {
                        Some(base_obj) => {
                            let base =
                                Self::parse_object(base_obj, resource_dicts, doc, cycle_count)?;
                            if matches!(base, Self::Pattern(..)) {
                                return Err(Error::InvalidColorSpaceNestedPattern);
                            }
                            Self::Pattern(Some(base.into()))
                        }
                        None => Self::Pattern(None),
                    },
                    b"Separation" => {
                        let Some(arr_el) = arr.get(1) else {
                            return Err(Error::InvalidColorSpace);
                        };
                        let separation_bytes = deref_name(arr_el, doc)?;

                        let separation_name = match separation_bytes {
                            b"All" => SeparationName::All,
                            b"None" => SeparationName::None,
                            other => {
                                SeparationName::Name(String::from_utf8_lossy(other).into_owned())
                            }
                        };
                        let alternate = Self::parse_object(
                            arr.get(2).ok_or(Error::InvalidColorSpace)?,
                            resource_dicts,
                            doc,
                            cycle_count,
                        )?;
                        Self::Separation(
                            Separation {
                                name: separation_name,
                                alternate,
                            }
                            .into(),
                        )
                    }
                    b"DeviceN" => {
                        let Some(arr_el) = arr.get(1) else {
                            return Err(Error::InvalidColorSpace);
                        };
                        let name_objects = deref_array(arr_el, doc)?;
                        let mut names: Vec<SeparationName> = Vec::new();
                        for name_obj in name_objects {
                            let name_bytes = deref_name(name_obj, doc)?;
                            let separation_name = match name_bytes {
                                b"All" => return Err(Error::InvalidColorSpace),
                                b"None" => SeparationName::None,
                                other => SeparationName::Name(
                                    String::from_utf8_lossy(other).into_owned(),
                                ),
                            };
                            names.push(separation_name);
                        }
                        let alternate = Self::parse_object(
                            arr.get(2).ok_or(Error::InvalidColorSpace)?,
                            resource_dicts,
                            doc,
                            cycle_count,
                        )?;
                        Self::DeviceN(DeviceN::try_new(names, alternate)?.into())
                    }

                    _ => return Err(Error::InvalidColorSpace),
                };
                Ok(cs)
            }
            _ => Err(Error::InvalidColorSpace),
        }
    }
}
