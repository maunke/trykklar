//! Graphics State Parameter Dictionaries
use crate::codec::TryFromObject;
use crate::dict::{DictKey, read_field, read_optional_field};
use crate::error::{Field, FieldExt, OptionalField};
use crate::font::Font;
use crate::text::TextFontSize;
use crate::unit::UserSpace;
use crate::{Error, GraphicsState, Length, ObjectAsF64, Result};
use lopdf::{Dictionary, Document, Object};

/// ISO 32000-1:2008 8.4.5 Table 58 – Entries in a Graphics State Parameter Dictionary
#[derive(Debug, Clone)]
pub struct ExtGState {
    // OP: sets stroking and non stroking to overprint, when op is not present
    pub(crate) overprint: OptionalField<Overprint>,
    // op
    pub(crate) non_stroking_overprint: OptionalField<NonStrokingOverprint>,
    // OPM
    pub(crate) overprint_mode: OptionalField<OverprintMode>,
    // Font
    pub(crate) font: OptionalField<ExtGStateFont>,
    // BM
    pub(crate) blend_mode: OptionalField<BlendMode>,
    // SMask
    pub(crate) soft_mask: OptionalField<SoftMask>,
    // CA (full transparent = 0.0)
    pub(crate) stroking_alpha: OptionalField<StrokingAlpha>,
    // ca
    pub(crate) non_stroking_alpha: OptionalField<NonStrokingAlpha>,
    // LW
    pub(crate) line_width: OptionalField<LineWidth>,
}

impl ExtGState {
    /// Resolves [`ExtGState`] from the key, resources and document.
    pub fn resolve<'a>(
        extgstate_key: &[u8],
        resource_dicts: &[&'a Dictionary],
        doc: &'a Document,
    ) -> Result<Self> {
        for d in resource_dicts {
            if let Ok(extgstate_dict_obj) = d.get_deref(b"ExtGState", doc) {
                let extgstate_dict = extgstate_dict_obj.as_dict()?;
                if let Ok(Object::Dictionary(extgstate_dict)) =
                    extgstate_dict.get_deref(extgstate_key, doc)
                {
                    let overprint = read_optional_field(doc, extgstate_dict);
                    let non_stroking_overprint = read_optional_field(doc, extgstate_dict);
                    let overprint_mode = read_optional_field(doc, extgstate_dict);
                    let font = read_optional_field(doc, extgstate_dict);
                    let blend_mode = read_optional_field(doc, extgstate_dict);
                    let soft_mask = read_optional_field(doc, extgstate_dict);
                    let stroking_alpha = read_optional_field(doc, extgstate_dict);
                    let non_stroking_alpha = read_optional_field(doc, extgstate_dict);
                    let line_width = read_optional_field(doc, extgstate_dict);
                    return Ok(Self {
                        overprint,
                        non_stroking_overprint,
                        overprint_mode,
                        font,
                        blend_mode,
                        soft_mask,
                        stroking_alpha,
                        non_stroking_alpha,
                        line_width,
                    });
                }
            }
        }
        Err(Error::ExtGStateNotFound(extgstate_key.into()))
    }

    /// ISO 32000-1:2008 8.4.5 Graphics State Parameter Dictionaries
    ///
    /// > Each entry in the parameter dictionary shall specify the value of an individual graphics
    /// > state parameter, as shown in Table 58. All entries need not be present for every
    /// > invocation of the gs operator; the supplied parameter dictionary may include any
    /// > combination of parameter entries. The results of gs shall be cumulative; parameter values
    /// > established in previous invocations persist until explicitly overridden.
    pub fn apply_to(self, gs: &mut GraphicsState) {
        // handle overprint
        match (self.overprint, self.non_stroking_overprint) {
            (Some(overprint), None) => {
                gs.stroking_overprint = overprint.clone().map(|op| op.get());
                gs.non_stroking_overprint = overprint.map(|op| op.get());
            }
            (Some(overprint), Some(non_stroking_overprint)) => {
                gs.stroking_overprint = overprint.map(|op| op.get());
                gs.non_stroking_overprint = non_stroking_overprint.map(|op| op.get());
            }
            (None, Some(non_stroking_overprint)) => {
                gs.non_stroking_overprint = non_stroking_overprint.map(|op| op.get());
            }
            (None, None) => (),
        }
        if let Some(overprint_mode) = self.overprint_mode {
            gs.overprint_mode = overprint_mode;
        }
        // font
        if let Some(font) = self.font {
            match font {
                Ok(f) => {
                    gs.text.font = Some(f.font);
                    gs.text.font_size = Some(f.size);
                }
                Err(e) => {
                    gs.text.font = Some(Err(e.clone()));
                    gs.text.font_size = Some(Err(e));
                }
            }
        }

        // blend mode
        if let Some(blend_mode) = self.blend_mode {
            gs.blend_mode = blend_mode;
        }

        // soft mask
        if let Some(soft_mask) = self.soft_mask {
            gs.soft_mask = soft_mask;
        }

        // alpha
        if let Some(stroking_alpha) = self.stroking_alpha {
            gs.stroking_alpha = stroking_alpha;
        }
        if let Some(non_stroking_alpha) = self.non_stroking_alpha {
            gs.non_stroking_alpha = non_stroking_alpha;
        }

        // line width
        if let Some(line_width) = self.line_width {
            gs.line_width = line_width;
        }
    }
}

/// `/OPM` Overprint Mode
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.3) The overprint mode (see 8.6.7, "Overprint Control").
///
/// 8.6.7:
///
/// > An additional graphics state parameter, the overprint mode (PDF 1.3), shall affect the
/// > interpretation of a tint value of 0.0 for a colour component in a DeviceCMYK colour space when
/// > overprinting is enabled. This parameter is controlled by the OPM entry in a graphics state
/// > parameter dictionary; it shall have an effect only when the overprint parameter is true, as
/// > described above.
///
/// So overprint mode plays a role only when the (alternate) ColorSpace is
/// [`crate::ColorSpace::DeviceCmyk`].
#[derive(Debug, Clone, Copy, Default)]
pub enum OverprintMode {
    /// `0`
    #[default]
    Standard,
    /// `1`
    NonZero,
}

impl DictKey for OverprintMode {
    const KEY: &'static [u8] = b"OPM";
}

impl TryFromObject<'_> for OverprintMode {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let value = obj.as_i64()?;
        let variant = match value {
            0 => Self::Standard,
            1 => Self::NonZero,
            _ => return Err(Error::InvalidPdfObject("OPM must be 0 or 1")),
        };
        Ok(variant)
    }
}

/// `/OP` Overprint Flag
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > A flag specifying whether to apply overprint (see 8.6.7, "Overprint Control"). In PDF 1.2 and
/// > earlier, there is a single overprint parameter that applies to all painting operations.
/// > Beginning with PDF 1.3, there shall be two separate overprint parameters: one for stroking and
/// > one for all other painting operations. Specifying an OP entry shall set both parameters unless
/// > there is also an op entry in the same graphics state parameter dictionary, in which case the
/// > OP entry shall set only the overprint parameter for stroking.
///
/// The last sentence is implemented in [`ExtGState::apply_to`].
#[derive(Debug, Clone, Copy)]
pub struct Overprint(bool);

impl Overprint {
    /// Returns the overprint flag.
    pub fn get(&self) -> bool {
        self.0
    }
}

impl DictKey for Overprint {
    const KEY: &'static [u8] = b"OP";
}

impl TryFromObject<'_> for Overprint {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let op = obj.as_bool()?;
        Ok(Self(op))
    }
}

/// `/op` Non Stroking Overprint Flag
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.3) A flag specifying whether to apply overprint (see 8.6.7, "Overprint
/// > Control") for painting operations other than stroking. If this entry is absent, the OP entry,
/// > if any, shall also set this parameter.
///
/// The last sentence is implemented in [`ExtGState::apply_to`].
#[derive(Debug, Clone, Copy)]
pub struct NonStrokingOverprint(bool);

impl NonStrokingOverprint {
    /// Returns the overprint flag.
    pub fn get(&self) -> bool {
        self.0
    }
}

impl DictKey for NonStrokingOverprint {
    const KEY: &'static [u8] = b"op";
}

impl TryFromObject<'_> for NonStrokingOverprint {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let op = obj.as_bool()?;
        Ok(Self(op))
    }
}

/// `/CA` Stroking Alpha
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.4) The current stroking alpha constant, specifying the constant shape or
/// > constant opacity value that shall be used for stroking operations in the transparent imaging
/// > model (see 11.3.7.2, "Source Shape and Opacity" and 11.6.4.4, "Constant Shape and Opacity").
///
/// ISO 32000-1:2008 11.3.7.2 Source Shape and Opacity
///
/// > All of the shape and opacity inputs shall have values in the range 0.0 to 1.0 (inclusive),
/// > with a default value of 1.0.
///
/// The range is not enforced in order to be lenient for a reader.
#[derive(Debug, Clone, Copy)]
pub struct StrokingAlpha(f64);

impl Default for StrokingAlpha {
    fn default() -> Self {
        Self(1.0)
    }
}

impl StrokingAlpha {
    /// Returns the stroking alpha value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl DictKey for StrokingAlpha {
    const KEY: &'static [u8] = b"CA";
}

impl TryFromObject<'_> for StrokingAlpha {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let op = obj.as_f64()?;
        Ok(Self(op))
    }
}

/// `/ca` Non Stroking Alpha
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.4) Same as CA, but for nonstroking operations.
///
/// See [`StrokingAlpha`] for more.
#[derive(Debug, Clone, Copy)]
pub struct NonStrokingAlpha(f64);

impl Default for NonStrokingAlpha {
    fn default() -> Self {
        Self(1.0)
    }
}

impl NonStrokingAlpha {
    /// Returns the non stroking alpha value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl DictKey for NonStrokingAlpha {
    const KEY: &'static [u8] = b"ca";
}

impl TryFromObject<'_> for NonStrokingAlpha {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let op = obj.as_f64()?;
        Ok(Self(op))
    }
}

/// `/LW` Line Width
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.3) The line width (see 8.4.3.2, "Line Width").
///
/// ISO 32000-1:2008 8.4.3.2 Line Width
///
/// > The line width parameter specifies the thickness of the line used to stroke a path. It shall
/// > be a non-negative number expressed in user space units [...].
///
/// The non-negative restriction is not enforced in order to be lenient for a reader.
#[derive(Debug, Clone, Copy)]
pub struct LineWidth(Length<UserSpace>);

impl LineWidth {
    /// Returns the lenghts of the line width in the [`UserSpace`].
    pub fn get(&self) -> Length<UserSpace> {
        self.0
    }

    pub(crate) fn from_raw(value: f64) -> Self {
        Self(Length::from_raw(value))
    }
}

impl DictKey for LineWidth {
    const KEY: &'static [u8] = b"LW";
}

impl TryFromObject<'_> for LineWidth {
    fn try_from_object(
        _doc: &'_ lopdf::Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let width = obj.as_f64()?;
        Ok(Self(Length::from_raw(width)))
    }
}

/// `/BM` Blend Mode
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.4) The current blend mode to be used in the transparent imaging model (see
/// > 11.3.5, "Blend Mode" and 11.6.3, "Specifying Blending Colour Space and Blend Mode").
///
/// ISO 32000-1:2008 11.3.5
/// - Table 136 Standard separable blend modes
/// - Table 137 standard nonseparable blend modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// > Selects the source colour, ignoring the backdrop.
    Normal,
    /// > Same as Normal. This mode exists only for compatibility and should not be used.
    Compatible,
    /// > Multiplies the backdrop and source colour values.
    Multiply,
    /// > Multiplies the complements of the backdrop and source colour values, then complements the
    /// > result.
    Screen,
    /// > Multiplies or screens the colours, depending on the backdrop colour value. Source colours
    /// > overlay the backdrop while preserving its highlights and shadows. The backdrop colour is
    /// > not replaced but is mixed with the source colour to reflect the lightness or darkness of
    /// > the backdrop.
    Overlay,
    /// > Selects the darker of the backdrop and source colours.
    Darken,
    /// > Selects the lighter of the backdrop and source colours.
    Lighten,
    /// > Brightens the backdrop colour to reflect the source colour. Painting with black produces
    /// > no changes.
    ColorDodge,
    /// > Darkens the backdrop colour to reflect the source colour. Painting with white produces no
    /// > change.
    ColorBurn,
    /// > Multiplies or screens the colours, depending on the source colour value. The effect is
    /// > similar to shining a harsh spotlight on the backdrop.
    HardLight,
    /// > Darkens or lightens the colours, depending on the source colour value. The effect is
    /// > similar to shining a diffused spotlight on the backdrop.
    SoftLight,
    /// > Subtracts the darker of the two constituent colours from the lighter colour: Painting with
    /// > white inverts the backdrop colour; painting with black produces no change.
    Difference,
    /// > Produces an effect similar to that of the Difference mode but lower in contrast. Painting
    /// > with white inverts the backdrop colour; painting with black produces no change.
    Exclusion,
    // Nonseparable
    /// > Creates a colour with the hue of the source colour and the saturation and luminosity of
    /// > the backdrop colour.
    Hue,
    /// > Creates a colour with the saturation of the source colour and the hue and luminosity of
    /// > the backdrop colour. Painting with this mode in an area of the backdrop that is a pure
    /// > gray (no saturation) produces no change.
    Saturation,
    /// > Creates a colour with the hue and saturation of the source colour and the luminosity of
    /// > the backdrop colour. This preserves the gray levels of the backdrop and is useful for
    /// > colouring monochrome images or tinting colour images.
    Color,
    /// > Creates a colour with the luminosity of the source colour and the hue and saturation of
    /// > the backdrop colour. This produces an inverse effect to that of the Color mode.
    Luminosity,
}

impl DictKey for BlendMode {
    const KEY: &'static [u8] = b"BM";
}

impl TryFromObject<'_> for BlendMode {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        match obj {
            Object::Name(name) => Self::try_from(name.as_slice()),
            Object::Array(array) => {
                for el in array {
                    let rslt = Self::try_from_object(_doc, _id, el);
                    if rslt.is_ok() {
                        return rslt;
                    }
                }
                Err(Error::InvalidPdfObject(
                    "Blend mode array does not contain valid blend modes",
                ))
            }
            _ => Err(Error::InvalidPdfObject("Blend mode must be name or array")),
        }
    }
}

impl TryFrom<&[u8]> for BlendMode {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let variant = match value {
            b"Normal" => Self::Normal,
            b"Compatible" => Self::Compatible,
            b"Multiply" => Self::Multiply,
            b"Screen" => Self::Screen,
            b"Overlay" => Self::Overlay,
            b"Darken" => Self::Darken,
            b"Lighten" => Self::Lighten,
            b"ColorDodge" => Self::ColorDodge,
            b"ColorBurn" => Self::ColorBurn,
            b"HardLight" => Self::HardLight,
            b"SoftLight" => Self::SoftLight,
            b"Difference" => Self::Difference,
            b"Exclusion" => Self::Exclusion,
            b"Hue" => Self::Hue,
            b"Saturation" => Self::Saturation,
            b"Color" => Self::Color,
            b"Luminosity" => Self::Luminosity,
            _ => return Err(Error::InvalidPdfObject("Blend mode name is invalid")),
        };
        Ok(variant)
    }
}

impl BlendMode {
    /// Blend Mode normal check for compatibility.
    ///
    /// Handles [`Self::Compatible`] case:
    ///
    /// ISO 32000-1:2008 11.3.5 Table 136 – Standard separable blend modes
    ///
    /// > `Compatible`: Same as Normal. This mode exists only for compatibility and should not be
    /// > used.
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal | Self::Compatible)
    }
}

/// `/SMask` Soft Mask
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.4) The current soft mask, specifying the mask shape or mask opacity values
/// > that shall be used in the transparent imaging model (see 11.3.7.2, "Source Shape and Opacity"
/// > and 11.6.4.3, "Mask Shape and Opacity"). Although the current soft mask is sometimes referred
/// > to as a “soft clip,” altering it with the gs operator completely replaces the old value with
/// > the new one, rather than intersecting the two as is done with the current clipping path
/// > parameter (see 8.5.4, "Clipping Path Operators").
#[derive(Debug, Clone, Default)]
pub enum SoftMask {
    /// ISO 32000-1:2008 11.6.4.3 Mask Shape and Opacity
    ///
    /// > The current soft mask parameter in the graphics state, set with the SMask entry in a
    /// > graphics state parameter dictionary, contains a soft-mask dictionary (see “Soft-Mask
    /// > Dictionaries”) defining the contents of the mask. The name None may be specified in place
    /// > of a soft-mask dictionary, denoting the absence of a soft mask. In this case, the mask
    /// > shape or opacity shall be implicitly 1.0 everywhere.
    #[default]
    None,
    /// Name Mask, see [`Mask`].
    Mask(Mask),
}

impl DictKey for SoftMask {
    const KEY: &'static [u8] = b"SMask";
}

impl TryFromObject<'_> for SoftMask {
    fn try_from_object(
        doc: &'_ Document,
        id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let obj = doc.dereference(obj)?.1;
        match obj {
            Object::Name(name) => match name.as_slice() {
                b"None" => Ok(Self::None),
                _ => Err(Error::InvalidPdfObject("Softmask name should be None")),
            },
            Object::Dictionary(_) => {
                let mask = Mask::try_from_object(doc, id, obj);
                mask.map(Self::Mask)
            }
            _ => Err(Error::InvalidPdfObject("Softmask must be a name or dict")),
        }
    }
}

/// ISO 32000-1:2008 11.6.5.2 Table 144 – Entries in a soft-mask dictionary
#[derive(Debug, Clone)]
pub struct Mask {
    sub_type: Field<MaskSubType>,
}

impl Mask {
    /// Returns the Mask subtype.
    pub fn sub_type(&self) -> Field<MaskSubType> {
        self.sub_type.as_field_ref().copied()
    }
}

impl TryFromObject<'_> for Mask {
    fn try_from_object(
        doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let obj = doc.dereference(obj)?;
        let dict = obj.1.as_dict()?;
        let sub_type = read_field(doc, dict);
        Ok(Self { sub_type })
    }
}

/// `/S` Mask Subtype       
///
/// ISO 32000-1:2008 11.6.5.2 Table 144 – Entries in a soft-mask dictionary
///
/// > (Required) A subtype specifying the method to be used in deriving the mask values from the
/// > transparency group specified by the G entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskSubType {
    /// `Alpha`
    ///
    /// > The group’s computed alpha shall be used, disregarding its colour (see “Deriving a Soft
    /// > Mask from Group Alpha”).
    Alpha,
    /// `Luminosity`
    ///
    /// > The group’s computed colour shall be converted to a single-component luminosity value (see
    /// > “Deriving a Soft Mask from Group Luminosity”).
    Luminosity,
}

impl DictKey for MaskSubType {
    const KEY: &'static [u8] = b"S";
}

impl TryFromObject<'_> for MaskSubType {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let variant = match obj.as_name()? {
            b"Alpha" => Self::Alpha,
            b"Luminosity" => Self::Luminosity,
            _ => {
                return Err(Error::InvalidPdfObject(
                    "MaskSubType must be one of Alpha or Luminosity",
                ));
            }
        };
        Ok(variant)
    }
}

/// `/Font` Font
///
/// ISO 32000-1:2008 8.4.5, Table 58:
///
/// > (Optional; PDF 1.3) An array of the form [ font, size ], where font shall be an indirect
/// > reference to a font dictionary and size shall be a number expressed in text space units. These
/// > two objects correspond to the operands of the Tf operator (see 9.3, "Text State Parameters and
/// > Operators"); however, the first operand shall be an indirect object reference instead of a
/// > resource name.
#[derive(Debug, Clone)]
pub struct ExtGStateFont {
    font: Result<Font>,
    size: Result<TextFontSize>,
}

impl DictKey for ExtGStateFont {
    const KEY: &'static [u8] = b"Font";
}

impl TryFromObject<'_> for ExtGStateFont {
    fn try_from_object(
        doc: &'_ Document,
        id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let obj = doc.dereference(obj)?.1;
        let [font_ref, size_obj] = &obj.as_array()?[..] else {
            return Err(Error::InvalidPdfObject(
                "Ext G State Font must be an array [font_ref, font_size]",
            ));
        };
        let font = Font::try_from_object(doc, id, font_ref);
        let size = TextFontSize::try_from_object(doc, id, size_obj);
        Ok(Self { font, size })
    }
}
