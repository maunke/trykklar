//! Fonts
use crate::codec::{ObjectAsF64, TryFromObject, deref_f64};
use crate::content::TryFromOperands;
use crate::dict::{DictKey, read_field, read_optional_field};
use crate::error::{Field, FieldError, FieldExt, OptionalField, OptionalFieldExt, ResultExt};
use crate::unit::UserSpace;
use crate::{Error, Matrix, Rect, Result, object_id};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::sync::Arc;

object_id!(FontId);

/// Font
///
/// An Arc wrapper around [`FontKind`] to have cheap clones.
#[derive(Debug, Clone)]
pub struct Font(Arc<FontKind>);

impl std::ops::Deref for Font {
    type Target = FontKind;
    fn deref(&self) -> &FontKind {
        &self.0
    }
}

impl TryFromObject<'_> for Font {
    fn try_from_object(doc: &'_ Document, id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let font = FontKind::try_from_object(doc, id, obj)?;
        Ok(Self(Arc::new(font)))
    }
}

impl Font {
    pub(crate) fn resolve<'a>(
        font_key: &[u8],
        resource_dicts: &[&'a Dictionary],
        doc: &'a Document,
    ) -> Result<Self> {
        let font_kind = FontKind::resolve(font_key, resource_dicts, doc)?;
        Ok(Self(Arc::new(font_kind)))
    }
}

/// Font Kind
///
/// ISO 32000-1:2008 9.5 Table 110 - Font Types
///
/// In 9.7.4 CIDFonts it stands:
///
/// > A CIDFont dictionary is a PDF object that contains information about a CIDFont program.
/// > Although its Type value is Font, a CIDFont is not actually a font. It does not have an
/// > Encoding entry, it may not be listed in the Font subdictionary of a resource dictionary, and
/// > it may not be used as the operand of the Tf operator. It shall be used only as a descendant of
/// > a Type 0 font. The CMap in the Type 0 font shall be what defines the encoding that maps
/// > character codes to CIDs in the CIDFont.
///
/// In Table 110 there is a further font type `CIDFont` that is not a member here, since CIDFont is
/// reachable from the content stream only through [`Type0Font::descendant`], see [`CidFont`] for
/// more.
#[derive(Debug, Clone)]
pub enum FontKind {
    /// Type 0
    ///
    /// > Subtypes: Type0
    Type0(Type0Font),
    /// Type 1
    ///
    /// > Subtypes: Type1, MMType1
    Type1(SimpleFont),
    /// Type 3
    ///
    /// > Subtypes: Type3
    Type3(Type3Font),
    /// TrueType
    ///
    /// > Subtypes: TrueType
    TrueType(SimpleFont),
}

impl TryFromObject<'_> for FontKind {
    fn try_from_object(doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let (id, font_obj) = doc.dereference(obj)?;
        let dict = font_obj.as_dict()?;
        Ok(match read_field::<FontSubtype>(doc, dict)? {
            FontSubtype::Type1 => FontKind::Type1(SimpleFont::try_from_object(doc, id, font_obj)?),
            FontSubtype::TrueType => {
                FontKind::TrueType(SimpleFont::try_from_object(doc, id, font_obj)?)
            }
            FontSubtype::Type3 => FontKind::Type3(Type3Font::try_from_object(doc, id, font_obj)?),
            FontSubtype::Type0 => FontKind::Type0(Type0Font::try_from_object(doc, id, font_obj)?),
        })
    }
}

impl FontKind {
    /// Returns the font matrix.
    pub(crate) fn font_matrix(&self) -> Result<FontMatrix> {
        match self {
            FontKind::Type3(f) => f.font_matrix.as_result(),
            _ => Ok(FontMatrix::default()),
        }
    }

    /// Checks if the font is embbeded.
    ///
    /// It is not checked, that every character used in the PDF is covered by the font.
    pub fn embedded(&self) -> Result<bool> {
        match self {
            FontKind::Type1(f) | FontKind::TrueType(f) => f.embedded(),
            FontKind::Type3(_) => Ok(true),
            FontKind::Type0(f) => f.descendant()?.embedded(),
        }
    }

    /// Returns the font bounding box.
    pub(crate) fn font_bbox(&self) -> Result<FontBBox> {
        match self {
            FontKind::Type1(f) | FontKind::TrueType(f) => f.font_bbox(),
            FontKind::Type3(f) => f.font_bbox.as_result(),
            FontKind::Type0(f) => f.descendant()?.font_bbox(),
        }
    }

    /// Returns the glyph width for a given code.
    pub fn glyph_width(&self, code: u32) -> Option<f64> {
        match self {
            FontKind::Type1(f) | FontKind::TrueType(f) => f.glyph_width(code),
            FontKind::Type3(f) => {
                resolve_glyph_width(f.first_char.as_ref().ok()?.0, f.widths.as_ref().ok()?, code)
            }
            FontKind::Type0(f) => f.descendant.as_ref().ok()?.width_for_cid(code),
        }
    }

    /// Decodes the string into glyph codes.
    ///
    /// Non-Identity Type0 CMaps aren't decoded here — fail closed.
    pub fn decode_string(&self, bytes: &[u8]) -> Result<Vec<u32>> {
        match self {
            FontKind::Type0(f) => match f.encoding()? {
                CMapEncoding::IdentityH | CMapEncoding::IdentityV => {
                    if !bytes.len().is_multiple_of(2) {
                        return Err(Error::InvalidPdfObject(
                            "Identity CMap: odd-length string operand",
                        ));
                    }
                    Ok(bytes
                        .chunks_exact(2)
                        .map(|c| u16::from_be_bytes([c[0], c[1]]) as u32)
                        .collect())
                }
                _ => Err(Error::Unsupported(
                    "Non Identity Type0 CMap is not supported",
                )),
            },
            _ => Ok(bytes.iter().map(|&b| b as u32).collect()),
        }
    }

    fn resolve<'a>(
        font_key: &[u8],
        resource_dicts: &[&'a Dictionary],
        doc: &'a Document,
    ) -> Result<Self> {
        for d in resource_dicts {
            let Ok(font_subdict) = d.get_deref(b"Font", doc) else {
                continue;
            };
            let Ok(font_dict) = font_subdict.as_dict() else {
                continue;
            };
            let Ok(font_entry) = font_dict.get(font_key) else {
                continue;
            };
            return Self::try_from_object(doc, None, font_entry);
        }
        Err(Error::FontNotFound(font_key.into()))
    }
}

/// Font Subtype
///
/// `/Subtype` key
///
/// Plese find the values in ISO 32000-1:2008:
///
/// - Type0: Table 121 – Entries in a Type 0 font dictionary
/// - Type1: Table 111 - Entries in a Type 1 font dictionary
/// - Type3: Table 112 - Entries in a Type 3 font dictionary
/// - TrueType: 9.6.3 TrueType Fonts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSubtype {
    /// Type0 font
    Type0,
    /// Type1 font
    Type1,
    /// Type3 font
    Type3,
    /// TrueType font
    TrueType,
}

impl DictKey for FontSubtype {
    const KEY: &'static [u8] = b"Subtype";
}

impl TryFromObject<'_> for FontSubtype {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Ok(match obj.as_name()? {
            b"Type1" | b"MMType1" => Self::Type1,
            b"TrueType" => Self::TrueType,
            b"Type3" => Self::Type3,
            b"Type0" => Self::Type0,
            _ => return Err(Error::InvalidPdfObject("unknown font subtype")),
        })
    }
}

/// Simple Font: covering type1 and truetype
///
/// ISO 32000-1:2008 Table 111 – Entries in a Type 1 font dictionary
#[derive(Debug, Clone)]
pub struct SimpleFont {
    pub(crate) id: FontId,
    pub(crate) base_font: Field<BaseFont>,
    pub(crate) standard: Option<StandardFont>,
    pub(crate) first_char: Field<FirstChar>,
    pub(crate) widths: Field<Widths>,
    pub(crate) descriptor: Field<FontDescriptor>,
}

impl SimpleFont {
    /// Returns the font object id.
    pub fn id(&self) -> FontId {
        self.id
    }

    /// Returns the base font.
    pub fn base_font(&self) -> Field<&BaseFont> {
        self.base_font.as_field_ref()
    }

    /// Returns the `StandardFont`, if the `BaseFont` describes a standard font.
    pub fn standard(&self) -> Option<&StandardFont> {
        self.standard.as_ref()
    }

    /// Returns the first char.
    pub fn first_char(&self) -> Field<FirstChar> {
        self.first_char.as_field_ref().copied()
    }

    /// Returns the widths.
    pub fn widths(&self) -> Field<&Widths> {
        self.widths.as_field_ref()
    }

    /// Returns the font descriptor.
    pub fn descriptor(&self) -> Field<&FontDescriptor> {
        self.descriptor.as_field_ref()
    }

    /// Checks if the font is embedded.
    pub fn embedded(&self) -> Result<bool> {
        match (&self.descriptor, &self.standard) {
            (Ok(d), _) => Ok(d.font_file_kind.is_some()),
            (Err(FieldError::Missing), Some(_)) => Ok(false),
            (Err(FieldError::Missing), None) => Err(Error::MissingField),
            (Err(FieldError::Invalid(e)), _) => Err(e.clone()),
        }
    }

    /// Returns the font bounding box.
    pub fn font_bbox(&self) -> Result<FontBBox> {
        let descriptor_font_bbox = match &self.descriptor {
            Ok(d) => d.font_bbox(),
            Err(e) => Err(e.clone()),
        };
        match (descriptor_font_bbox, &self.standard) {
            (Ok(bbox), _) => Ok(*bbox),
            (Err(FieldError::Missing), Some(sf)) => sf.font_bbox(),
            (Err(e), _) => Err(e.into()),
        }
    }

    /// Returns the glyph width.
    pub fn glyph_width(&self, code: u32) -> Option<f64> {
        resolve_glyph_width(
            self.first_char.as_ref().ok()?.0,
            self.widths.as_ref().ok()?,
            code,
        )
    }
}

impl TryFromObject<'_> for SimpleFont {
    fn try_from_object(doc: &'_ Document, id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let Some(id) = id else {
            return Err(Error::InvalidPdfObject("Font dict must have an object id"));
        };
        let id = FontId(id);
        let dict = obj.as_dict()?;
        let base_font = read_field::<BaseFont>(doc, dict);
        let standard = base_font
            .as_ref()
            .ok()
            .and_then(StandardFont::from_base_font);
        let first_char = read_field::<FirstChar>(doc, dict);
        let widths = read_field::<Widths>(doc, dict);
        let descriptor = read_field::<FontDescriptor>(doc, dict);
        Ok(Self {
            id,
            base_font,
            standard,
            first_char,
            widths,
            descriptor,
        })
    }
}

fn resolve_glyph_width(first_char: u32, widths: &Widths, code: u32) -> Option<f64> {
    widths
        .get()
        .get(code.checked_sub(first_char)? as usize)
        .copied()
}

/// Standard Type 1 Fonts
///
/// ISO 32000-1:2008 9.6.2.2 Standard Type 1 Fonts (Standard 14 Fonts)
///
/// > The PostScript names of 14 Type 1 fonts, known as the standard 14 fonts, are as follows:
/// > Times-Roman, Helvetica, Courier, Symbol, Times-Bold, Helvetica-Bold, Courier-Bold,
/// > ZapfDingbats, Times-Italic, Helvetica-Oblique, Courier-Oblique, Times-BoldItalic,
/// > Helvetica-BoldOblique, Courier-BoldOblique
/// >
/// > These fonts, or their font metrics and suitable substitution fonts, shall be available to the
/// > conforming reader.
#[derive(Debug, Clone)]
pub enum StandardFont {
    /// Times-Roman
    TimesRoman,
    /// Helvetica,
    Helvetica,
    /// Courier
    Courier,
    /// Symbol
    Symbol,
    /// Times-Bold
    TimesBold,
    /// Helvetica-Bold
    HelveticaBold,
    /// Courier-Bold
    CourierBold,
    /// ZapfDingbats
    ZapfDingbats,
    /// Times-Italic
    TimesItalic,
    /// Helvetica-Oblique
    HelveticaOblique,
    /// Courier-Oblique
    CourierOblique,
    /// Times-BoldItalic
    TimesBoldItalic,
    /// Helvetica-BoldOblique
    HelveticaBoldOblique,
    /// Courier-BoldOblique
    CourierBoldOblique,
}

impl StandardFont {
    fn from_base_font(base_font: &BaseFont) -> Option<Self> {
        Some(match base_font.0.as_str() {
            "Times-Roman" => Self::TimesRoman,
            "Helvetica" => Self::Helvetica,
            "Courier" => Self::Courier,
            "Symbol" => Self::Symbol,
            "Times-Bold" => Self::TimesBold,
            "Helvetica-Bold" => Self::HelveticaBold,
            "Courier-Bold" => Self::CourierBold,
            "ZapfDingbats" => Self::ZapfDingbats,
            "Times-Italic" => Self::TimesItalic,
            "Helvetica-Oblique" => Self::HelveticaOblique,
            "Courier-Oblique" => Self::CourierOblique,
            "Times-BoldItalic" => Self::TimesBoldItalic,
            "Helvetica-BoldOblique" => Self::HelveticaBoldOblique,
            "Courier-BoldOblique" => Self::CourierBoldOblique,
            _ => return None,
        })
    }
    /// Returns the Font BBox for the standard font.
    ///
    /// Adobe Core 14 AFM metrics, in 1000-unit glyph space.
    /// [Adobe Technical Note #5004](https://www.adobe.com/devnet/font.html)
    pub fn font_bbox(&self) -> Result<FontBBox> {
        let bbox = match self {
            Self::TimesRoman => [-168.0, -218.0, 1000.0, 898.0],
            Self::Helvetica => [-166.0, -225.0, 1000.0, 931.0],
            Self::Courier => [-23.0, -250.0, 715.0, 805.0],
            Self::Symbol => [-180.0, -293.0, 1090.0, 1010.0],
            Self::TimesBold => [-168.0, -218.0, 1000.0, 935.0],
            Self::HelveticaBold => [-170.0, -228.0, 1003.0, 962.0],
            Self::CourierBold => [-113.0, -250.0, 749.0, 801.0],
            Self::ZapfDingbats => [-1.0, -143.0, 981.0, 820.0],
            Self::TimesItalic => [-169.0, -217.0, 1010.0, 883.0],
            Self::HelveticaOblique => [-170.0, -225.0, 1116.0, 931.0],
            Self::CourierOblique => [-27.0, -250.0, 849.0, 805.0],
            Self::TimesBoldItalic => [-200.0, -218.0, 996.0, 921.0],
            Self::HelveticaBoldOblique => [-174.0, -228.0, 1114.0, 962.0],
            Self::CourierBoldOblique => [-57.0, -250.0, 869.0, 801.0],
        };
        Ok(FontBBox(Rect::try_from(bbox)?))
    }
}

/// Type 3 Font
///
/// ISO 32000-1:2008 Table 112 – Entries in a Type 3 font dictionary
#[derive(Debug, Clone)]
pub struct Type3Font {
    pub(crate) id: FontId,
    pub(crate) font_matrix: Field<FontMatrix>,
    pub(crate) first_char: Field<FirstChar>,
    pub(crate) widths: Field<Widths>,
    pub(crate) font_bbox: Field<FontBBox>,
}

impl Type3Font {
    /// Returns the font object id.
    pub fn id(&self) -> FontId {
        self.id
    }

    /// Returns the font matrix.
    pub fn font_matrix(&self) -> Field<&FontMatrix> {
        self.font_matrix.as_field_ref()
    }

    /// Returns the first char.
    pub fn first_char(&self) -> Field<FirstChar> {
        self.first_char.as_field_ref().copied()
    }

    /// Returns the widths.
    pub fn widths(&self) -> Field<&Widths> {
        self.widths.as_field_ref()
    }

    /// Returns the font bounding box.
    pub fn font_bbox(&self) -> Field<&FontBBox> {
        self.font_bbox.as_field_ref()
    }
}

impl<'a> TryFromObject<'a> for Type3Font {
    fn try_from_object(doc: &'a Document, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let Some(id) = id else {
            return Err(Error::InvalidPdfObject("Font dict must have an object id"));
        };
        let id = FontId(id);
        let dict = obj.as_dict()?;
        let font_matrix = read_field::<FontMatrix>(doc, dict);
        let first_char = read_field::<FirstChar>(doc, dict);
        let widths = read_field::<Widths>(doc, dict);
        let font_bbox = read_field::<FontBBox>(doc, dict);
        Ok(Self {
            id,
            font_matrix,
            first_char,
            widths,
            font_bbox,
        })
    }
}

/// Type 0 Font
///
/// ISO 32000-1:2008 Table 121 – Entries in a Type 0 font dictionary
#[derive(Debug, Clone)]
pub struct Type0Font {
    pub(crate) id: FontId,
    pub(crate) base_font: Field<BaseFont>,
    pub(crate) encoding: Field<CMapEncoding>,
    pub(crate) descendant: Field<DescendantFonts>,
}

impl Type0Font {
    /// Returns the font object id.
    pub fn id(&self) -> FontId {
        self.id
    }

    /// Returns the base font.
    ///
    /// > (Required) The name of the font. If the descendant is a Type 0 CIDFont, this name should
    /// > be the concatenation of the CIDFont’s BaseFont name, a hyphen, and the CMap name given in
    /// > the Encoding entry (or the CMapName entry in the CMap). If the descendant is a Type 2
    /// > CIDFont, this name should be the same as the CIDFont’s BaseFont name.
    pub fn base_font(&self) -> Field<&BaseFont> {
        self.base_font.as_field_ref()
    }

    /// Returns the CMap encoding.
    pub fn encoding(&self) -> Field<&CMapEncoding> {
        self.encoding.as_field_ref()
    }

    /// Returns the descenant Cid Font.
    pub fn descendant(&self) -> Field<&DescendantFonts> {
        self.descendant.as_field_ref()
    }
}

impl<'a> TryFromObject<'a> for Type0Font {
    fn try_from_object(doc: &'a Document, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let Some(id) = id else {
            return Err(Error::InvalidPdfObject("Font dict must have an object id"));
        };
        let id = FontId(id);
        let dict = obj.as_dict()?;
        let base_font = read_field(doc, dict);
        let encoding = read_field(doc, dict);
        let descendant = read_field(doc, dict);
        Ok(Self {
            id,
            base_font,
            encoding,
            descendant,
        })
    }
}

/// `/DescendantFonts`
///
/// ISO 32000-1:2008 Table 121 – Entries in a Type 0 font dictionary
///
/// > (Required) A one-element array specifying the CIDFont dictionary that is the descendant of
/// > this Type 0 font.
#[derive(Debug, Clone)]
pub struct DescendantFonts(CidFont);

impl DescendantFonts {
    fn embedded(&self) -> Result<bool> {
        self.0.embedded()
    }

    fn font_bbox(&self) -> Result<FontBBox> {
        self.0.font_bbox()
    }

    fn width_for_cid(&self, cid: u32) -> Option<f64> {
        self.0.width_for_cid(cid)
    }
}

impl DictKey for DescendantFonts {
    const KEY: &'static [u8] = b"DescendantFonts";
}

impl<'a> TryFromObject<'a> for DescendantFonts {
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let [cid] = &obj.as_array()?[..] else {
            return Err(Error::InvalidPdfObject(
                "Type0 font must have exactly one descendant",
            ));
        };
        let (cid_id, cid_obj) = doc.dereference(cid)?;
        Ok(Self(CidFont::try_from_object(doc, cid_id, cid_obj)?))
    }
}

/// CID Font
///
/// ISO 32000-1:2008 Table 117 – Entries in a CIDFont dictionary
#[derive(Debug, Clone)]
pub struct CidFont {
    pub(crate) id: FontId,
    pub(crate) subtype: Field<CidSubtype>,
    pub(crate) descriptor: Field<FontDescriptor>,
    pub(crate) default_width: Result<CidDefaultWidth>,
    pub(crate) widths: OptionalField<CidWidthMap>,
}

impl CidFont {
    /// Returns the id of the font object.
    pub fn id(&self) -> FontId {
        self.id
    }

    /// Returns the sub type.
    pub fn subtype(&self) -> Field<CidSubtype> {
        self.subtype.as_field_ref().copied()
    }

    /// Returns the cid default width.
    pub fn default_width(&self) -> Result<CidDefaultWidth> {
        self.default_width.ok_ref().copied()
    }

    /// Returns the slice of cid widths.
    pub fn widths(&self) -> OptionalField<&CidWidthMap> {
        self.widths.as_field_ref()
    }

    /// Returns the font desciptor.
    pub fn descriptor(&self) -> Field<&FontDescriptor> {
        self.descriptor.as_field_ref()
    }

    /// Checks, if the font is embedded.
    pub fn embedded(&self) -> Result<bool> {
        match &self.descriptor {
            Ok(d) => Ok(d.font_file_kind.is_some()),
            Err(e) => Err(e.clone().into()),
        }
    }

    /// `/FontDescriptor` → `/FontBBox`, in glyph space.
    pub fn font_bbox(&self) -> Result<FontBBox> {
        match &self.descriptor {
            Ok(d) => d.font_bbox.as_result(),
            Err(e) => Err(e.clone().into()),
        }
    }

    fn width_for_cid(&self, cid: u32) -> Option<f64> {
        if let Some(widths) = self.widths() {
            for w in widths.ok()?.get() {
                match w {
                    CidWidths::List { start, widths } => {
                        if let Some(i) = cid.checked_sub(*start)
                            && let Some(width) = widths.get(i as usize)
                        {
                            return Some(*width);
                        }
                    }
                    CidWidths::Range { start, end, width } => {
                        if cid >= *start && cid <= *end {
                            return Some(*width);
                        }
                    }
                }
            }
        }
        self.default_width().ok().map(|dw| dw.get())
    }
}

impl<'a> TryFromObject<'a> for CidFont {
    fn try_from_object(doc: &'a Document, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let Some(id) = id else {
            return Err(Error::InvalidPdfObject("cid font must have an object id"));
        };
        let id = FontId(id);
        let dict = obj.as_dict()?;
        let subtype = read_field(doc, dict);
        let default_width = match read_optional_field(doc, dict) {
            None => Ok(Default::default()),
            Some(dw) => dw,
        };
        let widths = read_optional_field(doc, dict);
        let descriptor = read_field(doc, dict);
        Ok(Self {
            id,
            subtype,
            default_width,
            widths,
            descriptor,
        })
    }
}

/// CID Font Subtype
///
/// ISO 32000-1:2008 Table 117 – Entries in a CIDFont dictionary
///
/// > (Required) The type of CIDFont shall be CIDFontType0 or CIDFontType2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CidSubtype {
    /// `CIDFontType0`
    Type0,
    /// `CIDFontType2`
    Type2,
}

impl DictKey for CidSubtype {
    const KEY: &'static [u8] = b"Subtype";
}

impl TryFromObject<'_> for CidSubtype {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Ok(match obj.as_name()? {
            b"CIDFontType0" => Self::Type0,
            b"CIDFontType2" => Self::Type2,
            _ => return Err(Error::InvalidPdfObject("unknown CIDFont subtype")),
        })
    }
}

/// Font descriptor
///
/// ISO 32000-1:2008 Table 117 – Entries in a CIDFont dictionary
///
/// `/FontDescriptor`
///
/// > (Required; shall be an indirect reference) A font descriptor describing the CIDFont’s default
/// > metrics other than its glyph widths (see 9.8, "Font Descriptors").
///
/// ISO 32000-1:2008 9.8 Font Descriptors
///
/// > A font descriptor specifies metrics and other attributes of a simple font or a CIDFont as a
/// > whole, as distinct from the metrics of individual glyphs. These font metrics provide
/// > information that enables a conforming reader to synthesize a substitute font or select a
/// > similar font when the font program is unavailable. The font descriptor may also be used to
/// > embed the font program in the PDF file.
///
/// > Font descriptors shall not be used with Type 0 fonts. Beginning with PDF 1.5, font descriptors
/// > may be used with Type 3 fonts.
///
/// ISO 32000-1:2008 Table 122 – Entries common to all font descriptors
#[derive(Debug, Clone)]
pub struct FontDescriptor {
    pub(crate) font_bbox: Field<FontBBox>,
    pub(crate) font_file_kind: Option<FontFileKind>,
}

impl FontDescriptor {
    /// Returns the font bounding box.
    pub fn font_bbox(&self) -> Field<&FontBBox> {
        self.font_bbox.as_field_ref()
    }

    /// Returns the font file kind.
    pub fn font_file_kind(&self) -> Option<FontFileKind> {
        self.font_file_kind
    }
}

impl DictKey for FontDescriptor {
    const KEY: &'static [u8] = b"FontDescriptor";
}

impl<'a> TryFromObject<'a> for FontDescriptor {
    fn try_from_object(doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let dict = obj.as_dict()?;
        let font_file_kind = if dict.has(b"FontFile") {
            Some(FontFileKind::Type1)
        } else if dict.has(b"FontFile2") {
            Some(FontFileKind::TrueType)
        } else if dict.has(b"FontFile3") {
            Some(FontFileKind::FontFile3)
        } else {
            None
        };
        let font_bbox = read_field(doc, dict);
        Ok(Self {
            font_bbox,
            font_file_kind,
        })
    }
}

/// Font File Kind
///
/// ISO 32000-1:2008 Table 126 – Embedded font organization for various font types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFileKind {
    /// Type 1 font
    Type1,
    /// Truetype font
    TrueType,
    /// Font file 3
    FontFile3,
}

/// Font bounding box
///
/// `/FontBBox`
///
/// ISO 32000-1:2008 Table 122 – Entries common to all font descriptors
///
/// > (Required, except for Type 3 fonts) A rectangle (see 7.9.5, "Rectangles"), expressed in the
/// > glyph coordinate system, that shall specify the font bounding box. This should be the smallest
/// > rectangle enclosing the shape that would result if all of the glyphs of the font were placed
/// > with their origins coincident and then filled.
#[derive(Debug, Clone, Copy)]
pub struct FontBBox(Rect<UserSpace>);

impl FontBBox {
    /// Returns the rectangle of the font bounding box.
    pub fn rect(&self) -> Rect<UserSpace> {
        self.0
    }
}

impl DictKey for FontBBox {
    const KEY: &'static [u8] = b"FontBBox";
}

impl<'a> TryFromObject<'a> for FontBBox {
    fn try_from_object(doc: &'a Document, id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let rect = Rect::try_from_object(doc, id, obj)?;
        let stated = [
            rect.origin.x.get(),
            rect.origin.y.get(),
            rect.size.width.get(),
            rect.size.height.get(),
        ];
        if stated.iter().all(|value| *value == 0.0) {
            return Err(Error::InvalidPdfObject(
                "FontBBox [0 0 0 0] states no glyph extent",
            ));
        }
        Ok(Self(rect))
    }
}

/// `/BaseFont`
///
/// Table 111 – Entries in a Type 1 font dictionary
///
/// > (Required) The PostScript name of the font. For Type 1 fonts, this is always the value of the
/// > FontName entry in the font program; for more information, see Section 5.2 of the PostScript
/// > Language Reference, Third Edition. The PostScript name of the font may be used to find the
/// > font program in the conforming reader or its environment. It is also the name that is used
/// > when printing to a PostScript output device.
#[derive(Debug, Clone)]
pub struct BaseFont(String);

impl BaseFont {
    /// Returns the font name.
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl DictKey for BaseFont {
    const KEY: &'static [u8] = b"BaseFont";
}

impl TryFromObject<'_> for BaseFont {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Ok(Self(String::from_utf8_lossy(obj.as_name()?).into_owned()))
    }
}

/// `/FirstChar`
///
/// Table 111 – Entries in a Type 1 font dictionary
///
/// > (Required except for the standard 14 fonts) The first character code defined in the font’s
/// > Widths array. Beginning with PDF 1.5, the special treatment given to the standard 14 fonts is
/// > deprecated. Conforming writers should represent all fonts using a complete font descriptor.
/// > For backwards capability, conforming readers shall still provide the special treatment
/// > identified for the standard 14 fonts.
#[derive(Debug, Clone, Copy)]
pub struct FirstChar(u32);

impl DictKey for FirstChar {
    const KEY: &'static [u8] = b"FirstChar";
}

impl TryFromObject<'_> for FirstChar {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Ok(Self(obj.as_i64()? as u32))
    }
}

/// `/Widths`
///
/// Table 111 – Entries in a Type 1 font dictionary
///
/// > (Required except for the standard 14 fonts; indirect reference preferred) An array of
/// > (LastChar − FirstChar + 1) widths, each element being the glyph width for the character code
/// > that equals FirstChar plus the array index. For character codes outside the range FirstChar to
/// > LastChar, the value of MissingWidth from the FontDescriptor entry for this font shall be used.
/// > The glyph widths shall be measured in units in which 1000 units correspond to 1 unit in text
/// > space. These widths shall be consistent with the actual widths given in the font program. For
/// > more information on glyph widths and other glyph metrics, see 9.2.4, "Glyph Positioning and
/// > Metrics".
/// >
/// > Beginning with PDF 1.5, the special treatment given to the standard 14 fonts is
/// > deprecated. Conforming writers should represent all fonts using a complete font descriptor.
/// > For backwards capability, conforming readers shall still provide the special treatment
/// > identified for the standard 14 fonts.
#[derive(Debug, Clone)]
pub struct Widths(Vec<f64>);

impl Widths {
    /// Returns the glyph widths.
    pub fn get(&self) -> &[f64] {
        &self.0
    }
}

impl DictKey for Widths {
    const KEY: &'static [u8] = b"Widths";
}

impl TryFromObject<'_> for Widths {
    fn try_from_object(doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        let widths = obj
            .as_array()?
            .iter()
            .map(|w| doc.dereference(w)?.1.as_f64())
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(widths))
    }
}

/// CID Default Width for glyphs
///
/// ISO 32000-1:2008 Table 117 – Entries in a CIDFont dictionary
///
/// `/DW`
///
/// > (Optional) The default width for glyphs in the CIDFont (see 9.7.4.3, "Glyph Metrics in
/// > CIDFonts").
/// >
/// > Default value: 1000 (defined in user units).
#[derive(Debug, Clone, Copy)]
pub struct CidDefaultWidth(f64);

impl Default for CidDefaultWidth {
    fn default() -> Self {
        Self(1000.)
    }
}

impl DictKey for CidDefaultWidth {
    const KEY: &'static [u8] = b"DW";
}

impl TryFromObject<'_> for CidDefaultWidth {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}

impl CidDefaultWidth {
    /// Returns the raw default width value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

/// Font Matrix
///
/// ISO 32000-1:2008 8.3.2.4 Other Coordinate Spaces
///
/// > Character glyphs in a font shall be defined in glyph space (see 9.2.4, "Glyph Positioning and
/// > Metrics"). The transformation from glyph space to text space shall be defined by the font
/// > matrix. For most types of fonts, this matrix shall be predefined to map 1000 units of glyph
/// > space to 1 unit of text space; for Type 3 fonts, the font matrix shall be given explicitly in
/// > the font dictionary (see 9.6.5, "Type 3 Fonts").
#[derive(Debug, Clone)]
pub struct FontMatrix(Matrix);

impl Default for FontMatrix {
    fn default() -> Self {
        Self(Matrix {
            a: 0.001,
            b: 0.0,
            c: 0.0,
            d: 0.001,
            e: 0.0,
            f: 0.0,
        })
    }
}

impl FontMatrix {
    /// Returns the transformation matrix.
    pub fn get(&self) -> &Matrix {
        &self.0
    }
}

impl DictKey for FontMatrix {
    const KEY: &'static [u8] = b"FontMatrix";
}

impl TryFromObject<'_> for FontMatrix {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Ok(Self(Matrix::try_from_operands(obj.as_array()?)?))
    }
}

/// Type0 CMap Encoding
///
/// `/Encoding`
///
/// ISO 32000-1:2008 Table 121
///
/// > (Required) The name of a predefined CMap, or a stream containing a CMap that maps character
/// > codes to font numbers and CIDs. If the descendant is a Type 2 CIDFont whose associated
/// > TrueType font program is not embedded in the PDF file, the Encoding entry shall be a
/// > predefined CMap name (see 9.7.4.2, "Glyph Selection in CIDFonts").
#[derive(Debug, Clone)]
pub enum CMapEncoding {
    /// `Identity-H`
    ///
    /// ISO 32000-1:2008 Table 118 - Predefined CJK Cmap names
    ///
    /// > The horizontal identity mapping for 2-byte CIDs; may be used with CIDFonts using any
    /// > Registry, Ordering, and Supplement values. It maps 2-byte character codes ranging from 0
    /// > to 65,535 to the same 2-byte CID value, interpreted high-order byte first.
    IdentityH,
    /// `Identity-V`
    ///
    /// ISO 32000-1:2008 Table 118 - Predefined CJK Cmap names
    ///
    /// > Vertical version of Identity-H. The mapping is the same as for Identity-H.
    IdentityV,
    /// Predefined CMap Encoding
    Named(String),
    /// Embedded CMap Encoding
    Embedded(ObjectId),
}

impl DictKey for CMapEncoding {
    const KEY: &'static [u8] = b"Encoding";
}

impl TryFromObject<'_> for CMapEncoding {
    fn try_from_object(_doc: &Document, id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        match obj {
            Object::Name(name) => Ok(match name.as_slice() {
                b"Identity-H" => CMapEncoding::IdentityH,
                b"Identity-V" => CMapEncoding::IdentityV,
                _ => CMapEncoding::Named(String::from_utf8_lossy(name).into_owned()),
            }),
            _ => id
                .map(CMapEncoding::Embedded)
                .ok_or(Error::InvalidPdfObject("invalid Type0 /Encoding")),
        }
    }
}

/// Cid Widths
///
/// ISO 32000-1:2008 9.7.4.3 Glyph Metrics in CIDFonts
///
/// > The W array allows the definition of widths for individual CIDs. The elements of the array
/// > shall be organized in groups of two or three, where each group shall be in one of these two
/// > formats:
#[derive(Debug, Clone)]
pub enum CidWidths {
    /// > `c [ w1 w2 ... wn ]`
    List {
        /// > In the first format, c shall be an integer specifying a starting CID value ...
        start: u32,
        /// > ...  it shall be followed by an array of n numbers that shall specify the widths for
        /// > n consecutive CIDs, starting with c.
        widths: Vec<f64>,
    },
    /// > `cfirst clast w`
    ///
    /// > The second format shall define the same width, w, for all CIDs in the range cfirst to
    /// > clast.
    Range {
        /// cfirst
        start: u32,
        /// clast
        end: u32,
        /// w
        width: f64,
    },
}

/// Cid Widths
///
/// `/W`
///
/// ISO 32000-1:2008 Table 117 – Entries in a CIDFont dictionary
///
/// > (Optional) A description of the widths for the glyphs in the CIDFont.
/// >
/// > NOTE: The array’s elements have a variable format that can specify individual widths for
/// > consecutive CIDs or one width for a range of CIDs (see 9.7.4.3, "Glyph Metrics in CIDFonts").
/// >
/// > Default value: none (the DW value shall be used for all glyphs).
#[derive(Debug, Clone, Default)]
pub struct CidWidthMap(Vec<CidWidths>);

impl CidWidthMap {
    /// Returns the slice of cid widths.
    pub fn get(&self) -> &[CidWidths] {
        &self.0
    }
}

impl DictKey for CidWidthMap {
    const KEY: &'static [u8] = b"W";
}

impl TryFromObject<'_> for CidWidthMap {
    fn try_from_object(doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        let arr = obj.as_array()?;
        let position_err = || Error::InvalidPdfObject("malformed CIDFont /W array");
        let array_el = |i: usize| arr.get(i).ok_or_else(position_err);

        let mut entries = Vec::new();
        let mut i = 0;
        while i < arr.len() {
            let start = doc.dereference(array_el(i)?)?.1.as_i64()? as u32;
            match doc.dereference(array_el(i + 1)?)?.1 {
                Object::Array(list) => {
                    let widths = list
                        .iter()
                        .map(|w| deref_f64(w, doc))
                        .collect::<Result<Vec<_>>>()?;
                    entries.push(CidWidths::List { start, widths });
                    i += 2;
                }
                end => {
                    let end = end.as_i64()? as u32;
                    let width = deref_f64(array_el(i + 2)?, doc)?;
                    entries.push(CidWidths::Range { start, end, width });
                    i += 3;
                }
            }
        }
        Ok(Self(entries))
    }
}
