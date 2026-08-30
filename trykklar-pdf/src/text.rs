//! Text
use crate::codec::TryFromObject;
use crate::content::TryFromOperands;
use crate::{Error, ObjectAsF64, Result};
use lopdf::{Document, Object, ObjectId};

/// Text rendering mode
///
/// ISO 32000-1:2008 9.3.6 Table 106 - Text rendering modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRenderMode {
    /// `0`: Fill text.
    Fill,
    /// `1`: Stroke text.
    Stroke,
    /// `2`: Fill, then stroke text.
    FillStroke,
    /// `3`: Neither fill nor stroke text (invisible).
    Invisible,
    /// `4`: Fill text and add to path for clipping.
    FillClip,
    /// `5`: Stroke text and add to path for clipping.
    StrokeClip,
    /// `6`: Fill, then stroke text and add to path for clipping.
    FillStrokeClip,
    /// `7`: Add text to path for clipping.
    Clip,
}

impl TextRenderMode {
    /// Checks, if the mode fills.
    pub fn fills(&self) -> bool {
        matches!(
            self,
            Self::Fill | Self::FillStroke | Self::FillClip | Self::FillStrokeClip
        )
    }

    /// Checks, if the mode strokes.
    pub fn strokes(&self) -> bool {
        matches!(
            self,
            Self::Stroke | Self::FillStroke | Self::StrokeClip | Self::FillStrokeClip
        )
    }
}

impl TryFrom<i64> for TextRenderMode {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        let mode = match value {
            0 => Self::Fill,
            1 => Self::Stroke,
            2 => Self::FillStroke,
            3 => Self::Invisible,
            4 => Self::FillClip,
            5 => Self::StrokeClip,
            6 => Self::FillStrokeClip,
            7 => Self::Clip,
            value => return Err(Error::InvalidTextRenderingMode { value }),
        };
        Ok(mode)
    }
}

/// Move Text
///
/// Operator: `Td`, Operands: `tx ty`
///
/// ISO 32000-1:2008 Table 108 – Text-positioning operators
///
/// > Move to the start of the next line, offset from the start of the current line by (tx , ty ).
/// > tx and ty shall denote numbers expressed in unscaled text space units.
#[derive(Debug, Clone)]
pub struct MoveText {
    tx: f64,
    ty: f64,
}

impl MoveText {
    /// Returns the x offset.
    pub fn tx(&self) -> f64 {
        self.tx
    }

    /// Returns the y offset.
    pub fn ty(&self) -> f64 {
        self.ty
    }
}

impl TryFromOperands for MoveText {
    fn try_from_operands(operands: &[lopdf::Object]) -> Result<Self> {
        let [x, y] = &operands else {
            return Err(Error::InvalidPdfObject("Td must have two operands"));
        };
        let tx = x.as_f64()?;
        let ty = y.as_f64()?;
        Ok(Self { tx, ty })
    }
}

/// Move text and set leading
///
/// Operator: `TD`, Operands: `tx ty`
///
/// ISO 32000-1:2008 Table 108 - Text-positioning operators
///
/// > Move to the start of the next line, offset from the start of the current line by (tx , ty ).
/// > As a side effect, this operator shall set the leading parameter in the text state.
#[derive(Debug, Clone)]
pub struct MoveTextSetLeading {
    tx: f64,
    ty: f64,
}

impl MoveTextSetLeading {
    /// Returns the x offset.
    pub fn tx(&self) -> f64 {
        self.tx
    }

    /// Returns the y offset.
    pub fn ty(&self) -> f64 {
        self.ty
    }
}

impl TryFromOperands for MoveTextSetLeading {
    fn try_from_operands(operands: &[lopdf::Object]) -> Result<Self> {
        let [x, y] = &operands else {
            return Err(Error::InvalidPdfObject("TD must have two operands"));
        };
        let tx = x.as_f64()?;
        let ty = y.as_f64()?;
        Ok(Self { tx, ty })
    }
}

/// Text Element containing text or text position adjustment
///
/// ISO 32000-1:2008 Table 109 - Text-Showing Operators
#[derive(Debug, Clone)]
pub enum TextElement {
    /// Text body
    Text(Box<[u8]>),
    /// Text position adjustment number
    ///
    /// Originates from TJ operator:
    ///
    /// > If it is a number, the operator shall adjust the text position by that amount; that is, it
    /// > shall translate the text matrix, Tm . The number shall be expressed in thousandths of a
    /// > unit of text space (see 9.4.4, "Text Space Details").
    Adjustment(f64),
}

impl TextElement {
    fn try_from_string_obj(obj: &lopdf::Object) -> Result<Self> {
        match obj {
            lopdf::Object::String(bytes, _) => Ok(Self::Text(bytes.clone().into_boxed_slice())),
            _ => Err(Error::InvalidOperands),
        }
    }
}

/// Show Text
///
/// This is an abstraction on top of all operators in ISO 32000-1:2008 "Table 109 Text-showing
/// operators", where each operator is resolved into.
#[derive(Debug, Clone)]
pub struct ShowText {
    elements: Box<[TextElement]>,
    next_line: bool,
    spacing: Option<(f64, f64)>,
}

impl ShowText {
    /// Returns the slice of text elements.
    pub fn elements(&self) -> &[TextElement] {
        &self.elements
    }

    /// Returns if text is moved to the line.
    pub fn next_line(&self) -> bool {
        self.next_line
    }

    /// Returns the tuple of word and char spacing.
    pub fn spacing(&self) -> Option<(f64, f64)> {
        self.spacing
    }

    /// Operator: `Tj`, Operands: `string`
    ///
    /// > Show a text string.
    pub fn try_from_tj_string(operands: &[lopdf::Object]) -> Result<Self> {
        let [s] = operands else {
            return Err(Error::InvalidOperands);
        };
        Ok(Self {
            elements: Box::new([TextElement::try_from_string_obj(s)?]),
            next_line: false,
            spacing: None,
        })
    }

    /// Operator: `TJ`, Operands: `array`
    ///
    /// > Show one or more text strings, allowing individual glyph positioning. Each element of
    /// > array shall be either a string or a number. If the element is a string, this operator
    /// > shall show the string. If it is a number, the operator shall adjust the text position by
    /// > that amount; that is, it shall translate the text matrix, Tm . The number shall be
    /// > expressed in thousandths of a unit of text space (see 9.4.4, "Text Space Details"). This
    /// > amount shall be subtracted from the current horizontal or vertical coordinate, depending
    /// > on the writing mode. In the default coordinate system, a positive adjustment has the
    /// > effect of moving the next glyph painted either to the left or down by the given amount.
    /// > Figure 46 shows an example of the effect of passing offsets to TJ.
    pub fn try_from_tj_array(operands: &[lopdf::Object]) -> Result<Self> {
        let [lopdf::Object::Array(arr)] = operands else {
            return Err(Error::InvalidOperands);
        };
        let elements = arr
            .iter()
            .map(|el| match el {
                lopdf::Object::String(bytes, _) => {
                    Ok(TextElement::Text(bytes.clone().into_boxed_slice()))
                }
                other => Ok(TextElement::Adjustment(other.as_f64()?)),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            elements: elements.into_boxed_slice(),
            next_line: false,
            spacing: None,
        })
    }

    /// Operator: `'`, Operands: `string`
    ///
    /// > Move to the next line and show a text string. This operator shall have the same effect as
    /// > the code:
    /// > ```text
    /// > T*
    /// > string Tj
    /// > ```
    pub fn try_from_single_quote(operands: &[lopdf::Object]) -> Result<Self> {
        let [s] = operands else {
            return Err(Error::InvalidOperands);
        };
        Ok(Self {
            elements: Box::new([TextElement::try_from_string_obj(s)?]),
            next_line: true,
            spacing: None,
        })
    }

    /// Operator: `"`, Operands: `aw ac string`
    ///
    /// > Move to the next line and show a text string, using aw as the word spacing and ac as the
    /// > character spacing (setting the corresponding parameters in the text state). aw and ac
    /// > shall be numbers expressed in unscaled text space units. This operator shall have the same
    /// > effect as this code:
    /// > ```text
    /// > aw Tw
    /// > ac Tc
    /// > string '
    /// > ```
    pub fn try_from_double_quote(operands: &[lopdf::Object]) -> Result<Self> {
        let [aw, ac, s] = operands else {
            return Err(Error::InvalidOperands);
        };
        Ok(Self {
            elements: Box::new([TextElement::try_from_string_obj(s)?]),
            next_line: true,
            spacing: Some((aw.as_f64()?, ac.as_f64()?)),
        })
    }
}

/// Text Font Size
///
/// ISO 32000-1:2008 9.3 Table 105 – Text state operators
///
/// > ... size shall be a number representing a scale factor. There is no initial value for either
/// > font or size.
#[derive(Debug, Clone, Copy)]
pub struct TextFontSize(f64);

impl TryFromObject<'_> for TextFontSize {
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}

impl TextFontSize {
    /// Returns the raw font size value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

/// Character spacing
///
/// ISO 32000-1:2008 9.3 Table 105 – Text state operators
///
/// > ... charSpace, which shall be a number expressed in unscaled text space units.
/// > Initial value: 0
#[derive(Debug, Clone)]
pub struct CharSpace(f64);

impl Default for CharSpace {
    fn default() -> Self {
        Self(0.0)
    }
}

impl CharSpace {
    /// Creates a char space.
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the raw char spacing value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl TryFromObject<'_> for CharSpace {
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}

/// Word Spacing
///
/// ISO 32000-1:2008 9.3 Table 105 – Text state operators
///
/// > ... wordSpace, which shall be a number expressed in unscaled text space units.
/// >
/// > Initial value: 0
#[derive(Debug, Clone)]
pub struct WordSpace(f64);

impl Default for WordSpace {
    fn default() -> Self {
        Self(0.0)
    }
}

impl WordSpace {
    /// Creates a word space.
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the raw word spacing value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl TryFromObject<'_> for WordSpace {
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}

/// Horizontal Scaling
///
/// ISO 32000-1:2008 9.3 Table 105 – Text state operators
///
/// > Set the horizontal scaling, Th , to (scale ÷ 100). scale shall be a number specifying the
/// > percentage of the normal width.
/// >
/// > Initial value: 100 (normal width).
#[derive(Debug, Clone)]
pub struct HorizontalScale(f64);

impl Default for HorizontalScale {
    fn default() -> Self {
        Self(100.0)
    }
}

impl HorizontalScale {
    /// Returns the raw horizontal scale value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl TryFromObject<'_> for HorizontalScale {
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}

/// Text Leading
///
/// ISO 32000-1:2008 9.3 Table 105 – Text state operators
///
/// > ...  leading, which shall be a number expressed in unscaled text space units.
/// >
/// > Initial value: 0
#[derive(Debug, Clone)]
pub struct TextLeading(f64);

impl Default for TextLeading {
    fn default() -> Self {
        Self(0.0)
    }
}

impl TextLeading {
    /// Creates text leading object.
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the raw text leading value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl TryFromObject<'_> for TextLeading {
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}

/// Text Rise
///
/// ISO 32000-1:2008 9.3 Table 105 – Text state operators
///
/// > ... rise, which shall be a number expressed in unscaled text space units.
/// >
/// > Initial value: 0
#[derive(Debug, Clone)]
pub struct TextRise(f64);

impl Default for TextRise {
    fn default() -> Self {
        Self(0.0)
    }
}

impl TextRise {
    /// Returns the raw text rise value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl TryFromObject<'_> for TextRise {
    fn try_from_object(_doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        Ok(Self(obj.as_f64()?))
    }
}
