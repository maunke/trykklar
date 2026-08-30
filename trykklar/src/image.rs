use crate::{Error, Result};
use pdf::{ContentWalkerStep, ImageXObject, Inch, Length};

/// Image Extension
pub trait Image {
    /// Returns the dots per inch of the painted image.
    fn dpi(&self, step: &ContentWalkerStep) -> Result<Dpi>;
}

impl Image for ImageXObject {
    fn dpi(&self, step: &ContentWalkerStep) -> Result<Dpi> {
        let ctm = step.graphics_state().ctm.as_ref()?;
        let user_unit = step.user_unit()?;
        let samples_width = self.width()?;
        let samples_height = self.height()?;
        let width: Length<Inch> = Length::try_from(ctm.a.hypot(ctm.b))?.to_physical(user_unit);
        let height: Length<Inch> = Length::try_from(ctm.c.hypot(ctm.d))?.to_physical(user_unit);

        let x = samples_width.get().get() as f64 / width.get();
        let y = samples_height.get().get() as f64 / height.get();

        if !x.is_finite() || !y.is_finite() {
            return Err(Error::NonFinite);
        }
        Ok(Dpi { x, y })
    }
}

/// Dots per inch.
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
