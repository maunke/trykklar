//! Operator Category
use crate::Operator;

/// Operator Category
///
/// ISO 32000-1:2008 Table 51 - Operator Categories
pub enum OperatorCategory {
    /// General graphics state
    ///
    /// > w, J, j, M, d, ri, i, gs
    /// > Table 57
    GeneralGState,
    /// Special graphics state
    ///
    /// > q, Q, cm
    /// > Table 57
    SpecialGState,
    /// Path construction
    ///
    /// > m, l, c, v, y, h, re
    /// > Table 59
    PathConstruction,
    /// Path painting
    ///
    /// > S, s, f, F, f*, B, B*, b, b*, n
    /// > Table 60
    PathPainting,
    /// Clipping paths
    ///
    /// > W, W*
    /// > Table 61
    ClippingPaths,
    /// Text objects
    ///
    /// > BT, ET
    /// > Tale 107
    TextObjects,
    /// Text state
    ///
    /// > Tc, Tw, Tz, TL, Tf, Tr, Ts
    TextState,
    /// Text positioning
    ///
    /// > Td, TD, Tm, T*
    /// > Table 108
    TextPositioning,
    /// Text showing
    ///
    /// > Tj, TJ, ', "
    /// > Table 109
    TextShowing,
    /// Type 3 fonts
    ///
    /// > d0, d1
    /// > Table 113
    Type3Fonts,
    /// Color
    ///
    /// > CS, cs, SC, SCN, sc, scn, G, g, RG, rg, K, k
    /// > Table 74
    Color,
    /// Shading patterns
    ///
    /// > sh
    /// > Table 77
    ShadingPatterns,
    /// Inline images
    ///
    /// > BI, ID, EI
    /// > Table 92
    InlineImages,
    /// XObjects
    ///
    /// > Do
    /// > Table 87
    XObjects,
    /// Marked content
    ///
    /// > MP, DP, BMC, BDC, EMC
    /// > Table 320
    MarkedContent,
    /// Compatibility
    ///
    /// > BX, EX
    /// > Table 32
    Compatibility,
}

impl From<&Operator<'_>> for OperatorCategory {
    fn from(value: &Operator<'_>) -> Self {
        match value {
            Operator::SetLineWidth(_)
            | Operator::SetLineCapStyle(_)
            | Operator::SetLineJoinStyle(_)
            | Operator::SetMiterLimit(_)
            | Operator::SetLineDashPattern(_)
            | Operator::SetRenderingIntent(_)
            | Operator::SetFlatnessTolerance(_)
            | Operator::SetExtGState(_) => Self::GeneralGState,
            Operator::SaveGraphicsState
            | Operator::RestoreGraphicsState
            | Operator::ModifyCtm(_) => Self::SpecialGState,
            Operator::ConstructPath(_) => Self::PathConstruction,
            Operator::PaintPath { .. } => Self::PathPainting,
            Operator::IntersectClip { .. } => Self::ClippingPaths,
            Operator::BeginText | Operator::EndText => Self::TextObjects,
            Operator::SetCharSpace(..)
            | Operator::SetWordSpace(..)
            | Operator::SetHorizontalScale(..)
            | Operator::SetTextLeading(..)
            | Operator::SetFontSize { .. }
            | Operator::SetTextRenderMode(..)
            | Operator::SetTextRise(..) => Self::TextState,
            Operator::MoveText(..)
            | Operator::MoveTextSetLeading(..)
            | Operator::SetTextMatrix(..)
            | Operator::NextLine => Self::TextPositioning,
            Operator::ShowText(..) => Self::TextShowing,
            Operator::SetGlyphWidth | Operator::SetGlyphWidthAndBBox => Self::Type3Fonts,
            Operator::SetStrokingColorSpace(..)
            | Operator::SetStrokingColor(..)
            | Operator::SetNonStrokingColorSpace(..)
            | Operator::SetNonStrokingColor(..) => Self::Color,
            Operator::PaintShading(..) => Self::ShadingPatterns,
            Operator::PaintXObject(..) => Self::XObjects,
            Operator::MarkedContentPoint
            | Operator::BeginMarkedContent { .. }
            | Operator::EndMarkedContent => Self::MarkedContent,
            Operator::BeginCompatibilitySection
            | Operator::EndCompatibilitySection
            | Operator::Unrecognized => Self::Compatibility,
        }
    }
}
