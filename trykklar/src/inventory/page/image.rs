use crate::{PaintedImage, WalkerProcessor};
use pdf::Operator;

/// Contains the painted images
#[derive(Debug, Default, Clone)]
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

    /// Returns the number of inderterminate painted image resolvings.
    pub fn inderterminate(&self) -> usize {
        self.inderterminate
    }
}

impl WalkerProcessor for ImagesInventory {
    fn process(&mut self, step: &pdf::ContentWalkerStep) {
        if let Operator::PaintXObject(Ok(pdf::XObject::Image(image))) = step.operator() {
            match PaintedImage::try_from_step(step, image.clone()) {
                Ok(painted_image) => self.images.push(painted_image),
                Err(_) => self.inderterminate += 1,
            }
        }
    }
}
