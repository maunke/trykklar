use crate::{Error, Result};
use pdf::page::PdfPageId;
use pdf::{
    BBox, ContentWalkerStep, ImageXObject, Inch, Length, PhysicalUnit, Rect, UserSpace, UserUnit,
};
use std::sync::Arc;

/// An image painted within a content stream.
#[derive(Debug, Clone)]
pub struct PaintedImage {
    page_id: PdfPageId,
    xobject: Arc<ImageXObject>,
    bbox: BBox<UserSpace>,
    user_unit: UserUnit,
    dpi: Dpi,
}

impl PaintedImage {
    pub(crate) fn try_from_step(
        step: &ContentWalkerStep<'_>,
        xobject: Arc<ImageXObject>,
    ) -> Result<Self> {
        let page_id = step.page_id();
        let ctm = step.graphics_state().ctm.as_ref()?;
        let bbox = step.painted_bbox()?;

        let user_unit = step.user_unit()?;
        let samples_width = xobject.width()?;
        let samples_height = xobject.height()?;
        let width: Length<Inch> = Length::try_from(ctm.a.hypot(ctm.b))?.to_physical(user_unit);
        let height: Length<Inch> = Length::try_from(ctm.c.hypot(ctm.d))?.to_physical(user_unit);

        let x = samples_width.get().get() as f64 / width.get();
        let y = samples_height.get().get() as f64 / height.get();

        if !x.is_finite() || !y.is_finite() {
            return Err(Error::NonFinite);
        }
        let dpi = Dpi { x, y };
        Ok(Self {
            page_id,
            xobject,
            bbox,
            user_unit,
            dpi,
        })
    }

    /// Returns the corresponding page id.
    pub fn page_id(&self) -> PdfPageId {
        self.page_id
    }

    /// Returns the bbox.
    pub fn bbox<U: PhysicalUnit>(&self) -> Option<Rect<U>> {
        self.bbox.into_rect().map(|r| r.to_physical(self.user_unit))
    }

    /// Returns the dots per inch of the painted image.
    pub fn dpi(&self) -> Dpi {
        self.dpi
    }

    /// Returns the underlying image xobject.
    pub fn xobject(&self) -> Arc<ImageXObject> {
        self.xobject.clone()
    }
}

/// Dots per inch.
#[derive(Debug, Clone, Copy)]
pub struct Dpi {
    x: f64,
    y: f64,
}

impl Dpi {
    /// X-axis dots per inch.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Y-axis dots per inch.
    pub fn y(&self) -> f64 {
        self.y
    }
}
