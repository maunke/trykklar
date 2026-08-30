use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::color::{Color, ColorSpace};
use crate::extgstate::{
    BlendMode, LineWidth, NonStrokingAlpha, OverprintMode, SoftMask, StrokingAlpha,
};
use crate::font::Font;
use crate::geometry::BBox;
use crate::text::{
    CharSpace, HorizontalScale, TextFontSize, TextLeading, TextRenderMode, TextRise, WordSpace,
};
use crate::unit::UserSpace;
use crate::{Error, Matrix, Result};

/// Color State containing the ColorSpace and Color
#[derive(Debug, Clone)]
pub struct ColorState<'a> {
    pub color_space: Result<ColorSpace>,
    pub color: Result<Color<'a>>,
}

impl<'a> Default for ColorState<'a> {
    fn default() -> Self {
        Self {
            color_space: Ok(ColorSpace::DeviceGray),
            color: Ok(Color::Values([1.0].into())),
        }
    }
}

/// Text State
///
/// ISO 32000-1:2008 9.3.1 Table 105 - Text state operators
#[derive(Debug, Clone)]
pub struct TextState {
    pub mode: Result<TextRenderMode>,
    pub font_size: Option<Result<TextFontSize>>,
    pub font: Option<Result<Font>>,
    pub char_space: Result<CharSpace>,
    pub word_space: Result<WordSpace>,
    pub horizontal_space: Result<HorizontalScale>,
    pub text_leading: Result<TextLeading>,
    pub text_rise: Result<TextRise>,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            mode: Ok(TextRenderMode::Fill),
            font_size: None,
            font: None,
            char_space: Ok(CharSpace::default()),
            word_space: Ok(WordSpace::default()),
            horizontal_space: Ok(HorizontalScale::default()),
            text_leading: Ok(TextLeading::default()),
            text_rise: Ok(TextRise::default()),
        }
    }
}

/// Graphics State
///
/// ISO 32000-1:2008 8.4 Graphics State
#[derive(Debug, Clone, Default)]
pub struct GraphicsState<'a>(Arc<InnerGraphicsState<'a>>);

impl<'a> Deref for GraphicsState<'a> {
    type Target = InnerGraphicsState<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for GraphicsState<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

#[derive(Debug, Clone)]
pub struct InnerGraphicsState<'a> {
    pub ctm: Result<Matrix>,
    pub clipping_bbox: Result<BBox<UserSpace>>,
    pub stroking: ColorState<'a>,
    pub non_stroking: ColorState<'a>,
    pub text: TextState,
    pub stroking_overprint: Result<bool>,
    pub non_stroking_overprint: Result<bool>,
    pub overprint_mode: Result<OverprintMode>,
    pub blend_mode: Result<BlendMode>,
    pub soft_mask: Result<SoftMask>,
    pub stroking_alpha: Result<StrokingAlpha>,
    pub non_stroking_alpha: Result<NonStrokingAlpha>,
    pub line_width: Result<LineWidth>,
}

impl Default for InnerGraphicsState<'_> {
    fn default() -> Self {
        Self {
            ctm: Ok(Matrix::IDENTITY),
            clipping_bbox: Ok(BBox::UNBOUNDED),
            stroking: ColorState::default(),
            non_stroking: ColorState::default(),
            text: TextState::default(),
            stroking_overprint: Ok(false),
            non_stroking_overprint: Ok(false),
            overprint_mode: Ok(OverprintMode::default()),
            blend_mode: Ok(BlendMode::Normal),
            soft_mask: Ok(SoftMask::default()),
            stroking_alpha: Ok(StrokingAlpha::default()),
            non_stroking_alpha: Ok(NonStrokingAlpha::default()),
            line_width: Ok(LineWidth::from_raw(1.0)),
        }
    }
}

impl InnerGraphicsState<'_> {
    // An error on parsing the ExtGState propagates as error to all states that can be part of
    // ExtGState.
    pub(crate) fn handle_ext_g_state_error(&mut self, e: &Error) {
        self.line_width = Err(e.clone());
        self.stroking_overprint = Err(e.clone());
        self.non_stroking_overprint = Err(e.clone());
        self.overprint_mode = Err(e.clone());
        self.blend_mode = Err(e.clone());
        self.soft_mask = Err(e.clone());
        self.stroking_alpha = Err(e.clone());
        self.non_stroking_alpha = Err(e.clone());
        self.text.font = Some(Err(e.clone()));
        self.text.font_size = Some(Err(e.clone()));
    }
}
