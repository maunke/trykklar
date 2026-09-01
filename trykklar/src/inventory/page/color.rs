use crate::WalkerProcessor;
use pdf::{Color, ColorSpace, Operator};
use std::collections::HashSet;

/// The set all all color spaces painted on a page.
#[derive(Default)]
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

    fn register_paint(&mut self, cs: &pdf::Result<ColorSpace>, color: &pdf::Result<Color>) {
        if let Ok(Color::Pattern(pdf::PatternColor::Shading(shading_pattern))) = color {
            match shading_pattern.shading() {
                Ok(shading) => match shading.color_space() {
                    Ok(color_space) => {
                        self.color_spaces.insert(color_space.clone());
                    }
                    Err(_) => self.inderterminate += 1,
                },
                Err(_) => self.inderterminate += 1,
            }
            return;
        }
        match cs {
            Ok(ColorSpace::Pattern(Some(color_space))) => {
                self.color_spaces.insert((**color_space).clone());
            }
            Ok(ColorSpace::Pattern(None)) => {}
            Ok(color_space) => {
                self.color_spaces.insert(color_space.clone());
            }
            Err(_) => self.inderterminate += 1,
        };
    }
}

impl WalkerProcessor for ColorSpacesInventory {
    fn process(&mut self, step: &pdf::ContentWalkerStep) {
        let gs = step.graphics_state();
        // check if painted bbox is ok and empty
        match step.painted_bbox() {
            Ok(painted_bbox) if painted_bbox.is_empty() => return,
            Err(_) => {
                self.inderterminate += 1;
                return;
            }
            _ => (),
        };
        // Match for graphics objects
        match step.operator() {
            Operator::PaintPath { fill, stroke } => {
                if *fill {
                    match gs.non_stroking_alpha {
                        Ok(alpha) if alpha.get() > 0. => self
                            .register_paint(&gs.non_stroking.color_space, &gs.non_stroking.color),
                        Err(_) => self.inderterminate += 1,
                        _ => (),
                    };
                }
                if *stroke {
                    match gs.stroking_alpha {
                        Ok(alpha) if alpha.get() > 0. => {
                            self.register_paint(&gs.stroking.color_space, &gs.stroking.color)
                        }
                        Err(_) => self.inderterminate += 1,
                        _ => (),
                    };
                }
            }
            Operator::ShowText(_) => {
                let (fill, stroke) = match gs.text.mode {
                    Ok(text_mode) => (text_mode.fills(), text_mode.strokes()),
                    Err(_) => {
                        self.inderterminate += 1;
                        return;
                    }
                };
                if fill {
                    match gs.non_stroking_alpha {
                        Ok(alpha) if alpha.get() > 0. => self
                            .register_paint(&gs.non_stroking.color_space, &gs.non_stroking.color),
                        Err(_) => self.inderterminate += 1,
                        _ => (),
                    };
                }
                if stroke {
                    match gs.stroking_alpha {
                        Ok(alpha) if alpha.get() > 0. => {
                            self.register_paint(&gs.stroking.color_space, &gs.stroking.color)
                        }
                        Err(_) => self.inderterminate += 1,
                        _ => (),
                    };
                }
            }
            Operator::PaintShading(shading) => match shading {
                Ok(s) => match s.color_space() {
                    Ok(color_space) => {
                        self.color_spaces.insert(color_space.clone());
                    }
                    Err(_) => self.inderterminate += 1,
                },
                Err(_) => self.inderterminate += 1,
            },
            Operator::PaintXObject(xobject) => match xobject {
                Ok(pdf::XObject::Image(image)) => match image.kind() {
                    Ok(pdf::ImageKind::Sampled { colorspace }) => {
                        self.color_spaces.insert(colorspace.clone());
                    }
                    Ok(pdf::ImageKind::Mask) => {
                        self.register_paint(&gs.non_stroking.color_space, &gs.non_stroking.color)
                    }
                    Err(_) => self.inderterminate += 1,
                },
                Ok(_) => {}
                Err(_) => self.inderterminate += 1,
            },
            _ => (),
        }
    }
}
