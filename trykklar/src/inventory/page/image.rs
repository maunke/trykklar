use crate::{Dpi, Image, WalkerProcessor};
use pdf::{BBox, ImageXObject, Operator, PhysicalUnit, Rect, UserSpace, UserUnit};
use std::sync::Arc;

pub struct PaintedImage {
    xobject: Arc<ImageXObject>,
    dpi: Dpi,
    bbox: BBox<UserSpace>,
    user_unit: UserUnit,
}

impl PaintedImage {
    /// Returns the image xobject.
    pub fn xobject(&self) -> &ImageXObject {
        &self.xobject
    }

    /// Returns the bounding box in physical space. `None`, when the bbox is empty.
    pub fn bbox<U: PhysicalUnit>(&self) -> Option<Rect<U>> {
        self.bbox.into_rect().map(|r| r.to_physical(self.user_unit))
    }

    /// Returns the dpi of the painted image.
    pub fn dpi(&self) -> Dpi {
        self.dpi
    }
}

/// Contains the painted images
#[derive(Default)]
pub struct ImagesInventory {
    images: Vec<PaintedImage>,
    inderterminate: usize,
}

impl ImagesInventory {
    /// Creates the images inventory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the painted images
    pub fn painted_images(&self) -> &[PaintedImage] {
        &self.images
    }

    /// Returns the number of inderterminate colorspace resolvings.
    pub fn inderterminate(&self) -> usize {
        self.inderterminate
    }
}

impl WalkerProcessor for ImagesInventory {
    fn process(&mut self, step: &pdf::ContentWalkerStep) {
        if let Operator::PaintXObject(Ok(pdf::XObject::Image(image))) = step.operator() {
            let Ok(bbox) = step.painted_bbox() else {
                self.inderterminate += 1;
                return;
            };
            let Ok(user_unit) = step.user_unit() else {
                self.inderterminate += 1;
                return;
            };
            let Ok(dpi) = image.dpi(step) else {
                self.inderterminate += 1;
                return;
            };
            let painted_image = PaintedImage {
                xobject: image.clone(),
                dpi,
                bbox,
                user_unit,
            };
            self.images.push(painted_image)
        }
    }
}
