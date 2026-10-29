use crate::WalkerProcessor;
use crate::inventory::step::PaintedColorSpaces;
use pdf::ColorSpace;
use std::collections::HashSet;

/// The set all all color spaces painted on a page.
#[derive(Debug, Default, Clone)]
pub struct ColorSpacesInventory {
    color_spaces: HashSet<ColorSpace>,
    inderterminate: usize,
}

impl ColorSpacesInventory {
    /// Creates a new empty colorspace inventory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the painted color spaces.
    pub fn color_spaces(&self) -> &HashSet<ColorSpace> {
        &self.color_spaces
    }

    /// Returns the number of inderterminate colorspace resolvings.
    pub fn inderterminate(&self) -> usize {
        self.inderterminate
    }
}

impl WalkerProcessor for ColorSpacesInventory {
    fn process(&mut self, step: &pdf::ContentWalkerStep) {
        match step.painted_color_spaces() {
            Ok(color_spaces) => {
                color_spaces.iter().for_each(|cs| {
                    self.color_spaces.insert(cs.clone());
                });
            }
            Err(_) => self.inderterminate += 1,
        }
    }
}
