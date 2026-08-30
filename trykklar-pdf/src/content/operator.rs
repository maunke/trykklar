//! Content Stream Operator

use crate::codec::TryFromObject;
use crate::color::{Color, ColorSpace, PatternColor};
use crate::content::{ResolvedCache, WalkerCache};
use crate::error::{ResourceKind, ResultExt};
use crate::extgstate::{ExtGState, LineWidth};
use crate::font::Font;
use crate::geometry::{
    CurveTo, CurveToControlCurrentTwo, CurveToControlOneThree, LineTo, MoveTo, PathElement,
};
use crate::ocg::Oc;
use crate::pattern::{Pattern, Shading, TilingPaintType};
use crate::text::{
    CharSpace, HorizontalScale, MoveText, MoveTextSetLeading, ShowText, TextFontSize, TextLeading,
    TextRenderMode, TextRise, WordSpace,
};
use crate::unit::UserSpace;
use crate::xobject::XObject;
use crate::{Error, Matrix, Rect, Result};
use lopdf::{Dictionary, Object};

/// Content Stream Operator
///
/// ISO 32000-1:2008 Table A.1 – PDF content stream operators
#[derive(Debug, Clone)]
pub enum Operator<'a> {
    /// > `q`: Save graphics state
    SaveGraphicsState,
    /// > `Q`: Restore graphics state
    RestoreGraphicsState,
    /// > `gs`: (PDF 1.2) Set parameters from graphics state parameter dictionary
    SetExtGState(Box<Result<ExtGState>>),
    /// > `cm`: Concatenate matrix to current transformation matrix
    ModifyCtm(Result<Matrix>),
    /// Paint Path Operator
    ///
    /// Covering ISO 32000-1:2008 8.5.3 Table 60 - "Path-Painting Operators" operators.
    PaintPath {
        /// Fills
        fill: bool,
        /// Strokes
        stroke: bool,
    },
    /// Show Text
    ///
    /// This is an abstraction on top of all operators in ISO 32000-1:2008 "Table 109 Text-showing
    /// operators", where each operator is resolved into.
    ShowText(Result<ShowText>),
    /// > `Tf`: Set text font and size
    SetFontSize {
        /// Text font
        font: Result<Font>,
        /// Text font size
        size: Result<TextFontSize>,
    },
    /// > `Tc`: Set character spacing
    SetCharSpace(Result<CharSpace>),
    /// > `Tw`: Set word spacing
    SetWordSpace(Result<WordSpace>),
    /// > `Tz`: Set horizontal text scaling
    SetHorizontalScale(Result<HorizontalScale>),
    /// > `TL`: Set text leading
    SetTextLeading(Result<TextLeading>),
    /// > `Ts`: Set text rise
    SetTextRise(Result<TextRise>),
    /// > `Tr`: Set text rendering mode
    SetTextRenderMode(Result<TextRenderMode>),
    /// Set Stroking ColorSpace
    ///
    /// ISO 32000-1:2008 8.6.8 Table 74 - Colour Operators
    SetStrokingColorSpace(Result<ColorSpace>, Result<Color<'a>>),
    /// Set Non-Stroking ColorSpace
    ///
    /// ISO 32000-1:2008 8.6.8 Table 74 - Colour Operators
    SetNonStrokingColorSpace(Result<ColorSpace>, Result<Color<'a>>),
    /// > `SC`: (PDF 1.1) Set color for stroking operations
    /// > `SCN`: (PDF 1.2) Set color for stroking operations (ICCBased and special colour spaces)
    SetStrokingColor(Result<Color<'a>>),
    /// > `sc`: (PDF 1.1) Set color for nonstroking operations
    /// > `scn`: (PDF 1.2) Set color for nonstroking operations (ICCBased and special colour spaces)
    SetNonStrokingColor(Result<Color<'a>>),
    /// > `w`: Set line width
    SetLineWidth(Result<LineWidth>),
    /// Construct Path
    ///
    /// An abstraction over ISO 32000-1:2008 8.5.2 Path Construction Operators Table 59.
    ConstructPath(Result<PathElement<UserSpace>>),
    /// > `Do`: Invoke named XObject
    PaintXObject(Result<XObject<'a>>),
    /// > `sh`: (PDF 1.3) Paint area defined by shading pattern
    PaintShading(Result<Shading>),
    /// > `BMC`: (PDF 1.2) Begin marked-content sequence
    /// > `BDC`: (PDF 1.2) Begin marked-content sequence with property list
    BeginMarkedContent {
        /// optional content tag
        oc: Result<Option<Result<Oc>>>,
    },
    /// > `EMC`: (PDF 1.2) End marked-content sequence
    EndMarkedContent,
    /// > `BT`: Begin text object
    BeginText,
    /// > `ET`: End text object
    EndText,
    /// > `Td`: Move text position
    MoveText(Result<MoveText>),
    /// > `TD`: Move text position and set leading
    MoveTextSetLeading(Result<MoveTextSetLeading>),
    /// > `T*`: Move to start of next text line
    NextLine,
    /// > `Tm`: Set text matrix and text line matrix
    SetTextMatrix(Result<Matrix>),
    /// Interset Clip
    ///
    /// Abstration over ISO 32000-1:2008 8.5.4 Table 61 - Clipping Path Operators
    IntersectClip {
        /// Using even-odd rule
        ///
        /// ISO 32000-1:2008 8.5.3.3.3 Even-Odd Rule
        even_odd: bool,
    },
    /// Other operators not covered yet.
    Other {
        /// Key
        key: String,
        /// Operands
        operands: Vec<Object>,
    },
}

impl<'a> Operator<'a> {
    pub(crate) fn resolve(
        operation: &lopdf::content::Operation,
        resource_dicts: &[&'a Dictionary],
        doc: &'a lopdf::Document,
        cache: &mut WalkerCache<'a>,
    ) -> Result<Self> {
        let operands = &operation.operands;
        let op = match operation.operator.as_str() {
            "Do" => Self::PaintXObject(match &operands[..] {
                [Object::Name(xobject_key)] => cache.xobject.get_or_resolve(xobject_key, || {
                    XObject::resolve(xobject_key, resource_dicts, doc)
                }),
                _ => Err(Error::InvalidOperands),
            }),
            // 8.4.4 Graphics State Operators
            "q" => Self::SaveGraphicsState,
            "Q" => Self::RestoreGraphicsState,
            "cm" => {
                let matrix = Matrix::try_from_operands(operands);
                Self::ModifyCtm(matrix)
            }
            "w" => Self::SetLineWidth(match &operands[..] {
                [w_obj] => LineWidth::try_from_object(doc, None, w_obj),
                _ => Err(Error::InvalidOperands),
            }),
            // 8.6.8 Table 74 - Colour Operators
            "CS" => {
                let cs =
                    ColorSpace::resolve(operation, resource_dicts, doc, &mut cache.color_space);
                let color = cs
                    .as_ref()
                    .map(ColorSpace::default_color)
                    .map_err(Clone::clone);
                Self::SetStrokingColorSpace(cs, color)
            }
            "G" | "RG" | "K" => {
                let cs =
                    ColorSpace::resolve(operation, resource_dicts, doc, &mut cache.color_space);
                let color = operands
                    .iter()
                    .map(|v| v.as_float().map_err(Error::from))
                    .collect::<Result<Vec<_>>>()
                    .map(|vals| Color::Values(vals.into_boxed_slice()));
                Self::SetStrokingColorSpace(cs, color)
            }
            "cs" => {
                let cs =
                    ColorSpace::resolve(operation, resource_dicts, doc, &mut cache.color_space);
                let color = cs
                    .as_ref()
                    .map(ColorSpace::default_color)
                    .map_err(Clone::clone);
                Self::SetNonStrokingColorSpace(cs, color)
            }
            "g" | "rg" | "k" => {
                let cs =
                    ColorSpace::resolve(operation, resource_dicts, doc, &mut cache.color_space);
                let color = operands
                    .iter()
                    .map(|v| v.as_float().map_err(Error::from))
                    .collect::<Result<Vec<_>>>()
                    .map(|vals| Color::Values(vals.into_boxed_slice()));
                Self::SetNonStrokingColorSpace(cs, color)
            }
            "SC" => Self::SetStrokingColor(
                operands
                    .iter()
                    .map(|v| v.as_float().map_err(Error::from))
                    .collect::<Result<Vec<_>>>()
                    .map(|vals| Color::Values(vals.into_boxed_slice())),
            ),
            "sc" => Self::SetNonStrokingColor(
                operands
                    .iter()
                    .map(|v| v.as_float().map_err(Error::from))
                    .collect::<Result<Vec<_>>>()
                    .map(|vals| Color::Values(vals.into_boxed_slice())),
            ),
            "SCN" => Self::SetStrokingColor(resolve_scn_color(
                operands,
                resource_dicts,
                doc,
                &mut cache.pattern,
            )),
            "scn" => Self::SetNonStrokingColor(resolve_scn_color(
                operands,
                resource_dicts,
                doc,
                &mut cache.pattern,
            )),
            // 8.7.4.2 Shading Operator
            "sh" => {
                let shading = match &operands[..] {
                    [Object::Name(shading_name)] => resource_dicts
                        .iter()
                        .find_map(|dict| {
                            let Ok(Object::Dictionary(shadings)) = dict.get_deref(b"Shading", doc)
                            else {
                                return None; // this dict has no /Shading — keep looking
                            };
                            let shading_obj = shadings.get_deref(shading_name, doc).ok()?; // name absent — keep looking
                            Some(Shading::resolve(shading_obj, resource_dicts, doc)) // found — first match wins
                        })
                        .unwrap_or(Err(Error::ResourceNotFound {
                            kind: ResourceKind::Shading,
                        })),
                    _ => Err(Error::InvalidOperands),
                };
                Self::PaintShading(shading)
            }
            // 8.5.3 Table 60 - Path-Painting Operators
            "S" | "s" => Self::PaintPath {
                fill: false,
                stroke: true,
            },
            "f" | "F" | "f*" => Self::PaintPath {
                fill: true,
                stroke: false,
            },
            "B" | "B*" | "b" | "b*" => Self::PaintPath {
                fill: true,
                stroke: true,
            },
            "n" => Self::PaintPath {
                fill: false,
                stroke: false,
            },
            // 8.5.2 Path Construction Operators Table 59
            "m" => {
                Self::ConstructPath(MoveTo::try_from_operands(operands).map(PathElement::MoveTo))
            }
            // lowecase L
            "l" => {
                Self::ConstructPath(LineTo::try_from_operands(operands).map(PathElement::LineTo))
            }
            "c" => {
                Self::ConstructPath(CurveTo::try_from_operands(operands).map(PathElement::CurveTo))
            }
            "v" => Self::ConstructPath(
                CurveToControlCurrentTwo::try_from_operands(operands)
                    .map(PathElement::CurveToControlCurrentTwo),
            ),
            "y" => Self::ConstructPath(
                CurveToControlOneThree::try_from_operands(operands)
                    .map(PathElement::CurveToControlOneThree),
            ),
            "re" => Self::ConstructPath(Rect::try_from_operands(operands).map(PathElement::Rect)),
            "h" => Self::ConstructPath(Ok(PathElement::Close)),
            // 9.4.3 Table 109 - Text-showing operators
            "Tj" => Operator::ShowText(ShowText::try_from_tj_string(operands)),
            "TJ" => Operator::ShowText(ShowText::try_from_tj_array(operands)),
            "'" => Operator::ShowText(ShowText::try_from_single_quote(operands)),
            "\"" => Operator::ShowText(ShowText::try_from_double_quote(operands)),
            // 9.3.1 Table 105 - Text state operators
            "Tr" => {
                let text_render_mode = match &operands[..] {
                    [mode] => match mode.as_i64() {
                        Ok(m) => TextRenderMode::try_from(m),
                        Err(e) => Err(Into::into(e)),
                    },
                    _ => Err(Error::InvalidOperands),
                };
                Self::SetTextRenderMode(text_render_mode)
            }
            "Tf" => match operands[..] {
                [Object::Name(ref font_key), ref size_obj] => {
                    let size = TextFontSize::try_from_object(doc, None, size_obj);
                    let font = cache
                        .font
                        .get_or_resolve(font_key, || Font::resolve(font_key, resource_dicts, doc));
                    Self::SetFontSize { font, size }
                }
                _ => Self::SetFontSize {
                    font: Err(Error::InvalidOperands),
                    size: Err(Error::InvalidOperands),
                },
            },
            "Tc" => {
                let tc = match &operands[..] {
                    [obj] => CharSpace::try_from_object(doc, None, obj),
                    _ => Err(Error::InvalidPdfObject("tc operator must have one operand")),
                };
                Self::SetCharSpace(tc)
            }
            "Tw" => {
                let tw = match &operands[..] {
                    [obj] => WordSpace::try_from_object(doc, None, obj),
                    _ => Err(Error::InvalidPdfObject("tw operator must have one operand")),
                };
                Self::SetWordSpace(tw)
            }
            "Tz" => {
                let tz = match &operands[..] {
                    [obj] => HorizontalScale::try_from_object(doc, None, obj),
                    _ => Err(Error::InvalidPdfObject("tz operator must have one operand")),
                };
                Self::SetHorizontalScale(tz)
            }
            "TL" => {
                let tl = match &operands[..] {
                    [obj] => TextLeading::try_from_object(doc, None, obj),
                    _ => Err(Error::InvalidPdfObject("tl operator must have one operand")),
                };
                Self::SetTextLeading(tl)
            }
            "Ts" => {
                let ts = match &operands[..] {
                    [obj] => TextRise::try_from_object(doc, None, obj),
                    _ => Err(Error::InvalidPdfObject("ts operator must have one operand")),
                };
                Self::SetTextRise(ts)
            } // 9.4 Table 107 - Text object operators
            "BT" => Self::BeginText,
            "ET" => Self::EndText,
            // 9.4 Table 108 - Text-positioning operators
            "Td" => Self::MoveText(MoveText::try_from_operands(operands)),
            "TD" => Self::MoveTextSetLeading(MoveTextSetLeading::try_from_operands(operands)),
            "T*" => Self::NextLine,
            "Tm" => Self::SetTextMatrix(Matrix::try_from_operands(operands)),
            // 14.6.1 Table 320 - Marked Content Operators
            // Only OC relevant information is considered
            // excluded: MP, DP
            "BMC" => Self::BeginMarkedContent { oc: Ok(None) },
            "BDC" => {
                let oc = match &operands[..] {
                    [Object::Name(tag), properties] => {
                        if tag == b"OC" {
                            Ok(Some(Oc::resolve(properties, resource_dicts, doc)))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => Err(Error::InvalidOperands),
                };
                Self::BeginMarkedContent { oc }
            }
            "EMC" => Self::EndMarkedContent,
            // 8.4.5 Graphics State Parameter Dictionaries
            "gs" => {
                let ext_g_state = match &operands[..] {
                    [Object::Name(extgstate_key)] => {
                        cache.ext_gstate.get_or_resolve(extgstate_key, || {
                            ExtGState::resolve(extgstate_key, resource_dicts, doc)
                        })
                    }
                    _ => Err(Error::InvalidOperands),
                };
                Self::SetExtGState(ext_g_state.into())
            }
            // 8.5.4 Table 61 - Clipping Path Operators
            "W" => Self::IntersectClip { even_odd: false },
            "W*" => Self::IntersectClip { even_odd: true },
            // Inline Image Support is missing on lopdf for decoding and encoding
            // https://github.com/J-F-Liu/lopdf/issues/78
            // This has a low prio, since inline images are not used in packaging design context.
            "BI" => return Err(Error::InlineImageUnsupported),
            key => Self::Other {
                key: key.to_string(),
                operands: operands.clone(),
            },
        };
        Ok(op)
    }
}

fn resolve_scn_color<'a>(
    operands: &[Object],
    resource_dicts: &[&'a Dictionary],
    doc: &'a lopdf::Document,
    cache: &mut ResolvedCache<Pattern<'a>>,
) -> Result<Color<'a>> {
    let mut scn_operands = operands.to_owned();
    let mut pattern = None;
    // check if a pattern name is set
    if let Some(scn_name) = operands.last()
        && let Object::Name(pattern_name) = scn_name
    {
        scn_operands.pop();
        pattern = Some(cache.get_or_resolve(pattern_name, || {
            Pattern::resolve(pattern_name, resource_dicts, doc)
        })?);
    }
    let mut values: Option<Box<[f32]>> = None;
    if !scn_operands.is_empty() {
        values = Some(
            scn_operands
                .iter()
                .map(|v| v.as_float().map_err(Error::from))
                .collect::<Result<Vec<_>>>()?
                .into_boxed_slice(),
        );
    }
    let color = match (pattern, values) {
        (None, Some(values)) => Color::Values(values),
        (Some(Pattern::Tiling(tiling_pattern)), None) => {
            match tiling_pattern.paint_type.ok_ref()? {
                TilingPaintType::Coloured => {
                    Color::Pattern(PatternColor::ColoredTiling(tiling_pattern))
                }
                TilingPaintType::Uncoloured => {
                    return Err(Error::InvalidPdfObject(
                        "uncoloured tiling in scn must have values",
                    ));
                }
            }
        }
        (Some(Pattern::Tiling(pattern)), Some(values)) => match pattern.paint_type.ok_ref()? {
            TilingPaintType::Coloured => {
                return Err(Error::InvalidPdfObject(
                    "coloured tiling in scn can not have values",
                ));
            }
            TilingPaintType::Uncoloured => {
                Color::Pattern(PatternColor::UncoloredTiling { pattern, values })
            }
        },
        (Some(Pattern::Shading(pattern)), None) => Color::Pattern(PatternColor::Shading(pattern)),
        _ => return Err(Error::InvalidPdfObject("scn operands are not correct")),
    };
    Ok(color)
}

pub(crate) trait TryFromOperands: Sized {
    fn try_from_operands(operands: &[Object]) -> Result<Self>;
}
