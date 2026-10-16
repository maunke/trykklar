//! Renderer for pages and images
use crate::{Dpi, Error, PaintedImage, Result};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::vello_cpu::color::Rgba8;
use hayro::{PixmapSettings, RenderCache, RenderSettings, hayro_syntax, render};
use pdf::content::write_operator::{ModifyCtm, PaintXObject, WriteOperation, WriteOperations};
use pdf::geometry::{Point, Size};
use pdf::optional_content::OCPropertiesMut;
use pdf::page::{CropBox, MediaBox, PageRotate};
use pdf::xobject::ImageXObjectMut;
use pdf::{Inch, OcConfig, Pdf, Rect, UserUnit};
use std::num::NonZeroU32;

/// RGBA Background color
#[derive(Debug, Copy, Clone)]
pub struct BackgroundColor {
    components: [u8; 4],
}

impl BackgroundColor {
    /// Creates a new rgba background color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from([r, g, b, a])
    }

    /// Transparent.
    pub const TRANSPARENT: Self = Self {
        components: [0, 0, 0, 0],
    };

    /// White.
    pub const WHITE: Self = Self {
        components: [255, 255, 255, 255],
    };

    /// Black.
    pub const BLACK: Self = Self {
        components: [0, 0, 0, 255],
    };
}

impl Default for BackgroundColor {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl From<BackgroundColor> for Rgba8 {
    fn from(value: BackgroundColor) -> Self {
        let [r, g, b, a] = value.components;
        Self { r, g, b, a }
    }
}

impl From<[u8; 4]> for BackgroundColor {
    fn from(components: [u8; 4]) -> Self {
        Self { components }
    }
}

/// Pixmap
///
/// This serves as an abstraction for further supported colorspaces and formats. Right now it
/// contains rgba png data only.
pub struct Pixmap {
    png: Vec<u8>,
}

impl Pixmap {
    /// Returns the png data.
    pub fn as_png(&self) -> &[u8] {
        &self.png
    }
}

/// Set dpi and background for the renderer.
#[derive(Debug, Clone, Copy)]
pub struct RendererSettings {
    dpi: Dpi,
    background: BackgroundColor,
}

impl RendererSettings {
    /// Creates a new renderer setting.
    pub fn new(dpi: Dpi, background: BackgroundColor) -> Self {
        Self { dpi, background }
    }
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            dpi: Dpi::new(150., 150.),
            background: BackgroundColor::default(),
        }
    }
}

impl RendererSettings {
    fn into_pixmap_settings(self, user_unit: UserUnit) -> PixmapSettings {
        const PT_TO_INCH_FACTOR: f64 = 1. / 72.;
        let x_scale = (user_unit.get() * self.dpi.x() * PT_TO_INCH_FACTOR) as f32;
        let y_scale = (user_unit.get() * self.dpi.y() * PT_TO_INCH_FACTOR) as f32;
        let bg_color: Rgba8 = self.background.into();
        PixmapSettings {
            x_scale,
            y_scale,
            bg_color: bg_color.into(),
        }
    }
}

/// Renderer
///
/// The Trykklar struct is owned and not borrowed since the underlying PDF needs to be mutated in
/// order to change at least oc properties for oc aware rendering.
pub struct Renderer {
    pdf: Pdf,
    settings: RendererSettings,
    interpreter_settings: InterpreterSettings,
    renderer_settings: RenderSettings,
    renderer_pdf: hayro_syntax::Pdf,
}

impl Renderer {
    /// Creates a renderer with the oc context of the provided underlying PDF.
    pub fn try_new(mut pdf: Pdf) -> Result<Self> {
        let renderer_pdf = Self::pdf_to_renderer_pdf(&mut pdf)?;
        Ok(Self {
            pdf,
            settings: RendererSettings::default(),
            interpreter_settings: InterpreterSettings::default(),
            renderer_settings: RenderSettings::default(),
            renderer_pdf,
        })
    }

    fn pdf_to_renderer_pdf(pdf: &mut Pdf) -> Result<hayro_syntax::Pdf> {
        let mut pdf_data = Vec::new();
        pdf.save_to(&mut pdf_data)?;
        Ok(hayro_syntax::Pdf::new(pdf_data)?)
    }

    /// Sets the oc config of the PDF and returns a new renderer instance.
    pub fn with_oc_config(mut self, config: OcConfig) -> Result<Self> {
        let mut oc_props = OCPropertiesMut::try_from(&mut self.pdf)?;
        oc_props.set_default_config(config.clone())?;
        self.renderer_pdf = Self::pdf_to_renderer_pdf(&mut self.pdf)?;
        Ok(self)
    }

    /// Sets the renderer settings.
    pub fn with_settings(mut self, settings: RendererSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Returns the page pixmap.
    pub fn page(&self, number: u32) -> Result<Pixmap> {
        let user_unit = self.pdf.page(number)?.user_unit()?;
        let page = self
            .renderer_pdf
            .pages()
            .get(number as usize)
            .ok_or(Error::RendererPageNotFound)?;
        self.pixmap_from_page(user_unit, page, &self.settings)
    }

    fn pixmap_from_page(
        &self,
        user_unit: UserUnit,
        page: &hayro_syntax::page::Page,
        settings: &RendererSettings,
    ) -> Result<Pixmap> {
        let cache = RenderCache::new();
        let png = render(
            page,
            &cache,
            &self.interpreter_settings,
            &self.renderer_settings,
            &settings.into_pixmap_settings(user_unit),
        )
        .into_png()
        .map_err(|_| Error::RendererPng)?;
        Ok(Pixmap { png: png.clone() })
    }

    /// Returns the [`Pixmap`] of provided [`PaintedImage`].
    ///
    /// Use [`Self::painted_image_with_max_dimensions`] to render with a max width or height.
    pub fn painted_image(&self, image: &PaintedImage) -> Result<Pixmap> {
        self.render_painted_image(image, None, None)
    }

    /// Returns [`Pixmap`] of provided [`PaintedImage`] with max dimensions for the width and
    /// height.
    pub fn painted_image_with_max_dimensions(
        &self,
        image: &PaintedImage,
        max_width: NonZeroU32,
        max_height: NonZeroU32,
    ) -> Result<Pixmap> {
        self.render_painted_image(image, Some(max_width), Some(max_height))
    }

    // Internally an image is renderer by replacing the page content with a ctm and paint image
    // xobject operator in order to span the whole page that is set to the dimensions of the image.
    //
    // This approach is chosen in order to render all raw image xobject data related information
    // like colorspace or mask out of the box.
    fn render_painted_image(
        &self,
        image: &PaintedImage,
        max_width: Option<NonZeroU32>,
        max_height: Option<NonZeroU32>,
    ) -> Result<Pixmap> {
        let page_id = image.page_id();
        let image_id = image.xobject().id();
        let mut pdf = self.pdf.clone();

        // Remove the oc entry from xobject to prevent to be hidden when rendered.
        ImageXObjectMut::try_new(&mut pdf, image_id)?.remove_oc()?;

        let paint_xobject = PaintXObject::try_from_image(image_id, &pdf)?;
        let number = pdf.page_number_by_id(page_id)?;
        let mut page = pdf.page_mut(number)?;

        page.set_rotation(PageRotate::R0)?;
        page.set_user_unit(UserUnit::new(1.0))?;

        let width = u32::from(image.xobject().width()?.get()) as f64;
        let height = u32::from(image.xobject().height()?.get()) as f64;

        let size = Size {
            width: (width / 72.).try_into()?,
            height: (height / 72.).try_into()?,
        };
        let origin = Point::ORIGIN;
        let media_box: MediaBox<Inch> = Rect { size, origin }.into();
        let crop_box: CropBox<Inch> = Rect { size, origin }.into();

        page.set_media_box(media_box)?;
        page.set_crop_box(crop_box)?;

        let modify_ctm = ModifyCtm::new(pdf::Matrix {
            a: width,
            b: 0.,
            c: 0.,
            d: height,
            e: 0.,
            f: 0.,
        });
        let operations: WriteOperations = vec![
            WriteOperation::new(modify_ctm),
            WriteOperation::new(paint_xobject),
        ];
        page.set_content(operations)?;

        let user_unit = page.as_page()?.user_unit()?;
        let renderer_pdf = Self::pdf_to_renderer_pdf(&mut pdf)?;
        let page = renderer_pdf
            .pages()
            .get(number as usize)
            .ok_or(Error::RendererPageNotFound)?;

        // Compute scaling
        let scale_w = if let Some(max_width) = max_width {
            width / u32::from(max_width) as f64
        } else {
            1.0
        };
        let scale_h = if let Some(max_height) = max_height {
            height / u32::from(max_height) as f64
        } else {
            1.0
        };
        let scale = scale_w.max(scale_h);
        let dpi_val = 72. / scale;
        let dpi = Dpi::new(dpi_val, dpi_val);
        let mut settings = self.settings;
        settings.dpi = dpi;

        self.pixmap_from_page(user_unit, page, &settings)
    }
}
