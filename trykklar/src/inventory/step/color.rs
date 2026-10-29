use crate::Result;
use pdf::color::ColorantName;
use pdf::error::ResultExt;
use pdf::{Color, ColorSpace, ContentWalkerStep, Operator};
use std::collections::HashSet;

/// Resolves painted ColorSpaces
pub trait PaintedColorSpaces {
    /// Returns the painted color spaces.
    ///
    /// If the painted bbox is empty, an empty hashset is returned since nothing is painted.
    fn painted_color_spaces(&self) -> Result<HashSet<ColorSpace>>;
}

impl PaintedColorSpaces for ContentWalkerStep<'_> {
    fn painted_color_spaces(&self) -> Result<HashSet<ColorSpace>> {
        let mut color_spaces = HashSet::new();
        let painted_bbox = self.painted_bbox()?;
        if painted_bbox.is_empty() {
            return Ok(color_spaces);
        }
        let gs = self.graphics_state();

        let mut register_paint =
            |cs: &pdf::Result<ColorSpace>, color: &pdf::Result<Color>| -> Result<()> {
                if let Ok(Color::Pattern(pdf::PatternColor::Shading(shading_pattern))) = color {
                    color_spaces.insert(shading_pattern.shading()?.color_space()?.clone());
                }
                match cs.ok_ref()? {
                    ColorSpace::Pattern(Some(color_space)) => {
                        color_spaces.insert((**color_space).clone());
                    }

                    ColorSpace::Pattern(None) => {}
                    color_space => {
                        color_spaces.insert(color_space.clone());
                    }
                };
                Ok(())
            };

        match self.operator() {
            Operator::PaintPath { fill, stroke } => {
                if *fill && gs.non_stroking_alpha.ok_ref()?.get() > 0. {
                    register_paint(&gs.non_stroking.color_space, &gs.non_stroking.color)?;
                }
                if *stroke && gs.stroking_alpha.ok_ref()?.get() > 0. {
                    register_paint(&gs.stroking.color_space, &gs.stroking.color)?;
                }
            }
            Operator::ShowText(_) => {
                let text_mode = gs.text.mode.ok_ref()?;
                if text_mode.fills() && gs.non_stroking_alpha.ok_ref()?.get() > 0. {
                    register_paint(&gs.non_stroking.color_space, &gs.non_stroking.color)?;
                }
                if text_mode.strokes() && gs.stroking_alpha.ok_ref()?.get() > 0. {
                    register_paint(&gs.stroking.color_space, &gs.stroking.color)?;
                }
            }
            Operator::PaintShading(shading) => {
                color_spaces.insert(shading.ok_ref()?.color_space()?.clone());
            }
            Operator::PaintXObject(xobject) => {
                if let pdf::XObject::Image(image) = xobject.ok_ref()? {
                    match image.kind().ok_ref()? {
                        pdf::ImageKind::Sampled { colorspace } => {
                            color_spaces.insert(colorspace.clone());
                        }
                        pdf::ImageKind::Mask => {
                            register_paint(&gs.non_stroking.color_space, &gs.non_stroking.color)?;
                        }
                    }
                }
            }
            _ => (),
        };
        Ok(color_spaces)
    }
}

/// Resolves painted Colorants
pub trait ColorSpaceColorantNames {
    /// Returns the colorant names.
    fn colorant_names(&self) -> HashSet<ColorantName>;
}

impl ColorSpaceColorantNames for HashSet<ColorSpace> {
    fn colorant_names(&self) -> HashSet<ColorantName> {
        fn resolve_colorant_name_from_cs(cs: &ColorSpace, set: &mut HashSet<ColorantName>) {
            match cs {
                ColorSpace::Separation(separation) => {
                    set.insert(separation.name().clone());
                }
                ColorSpace::DeviceN(device_n) => {
                    for name in device_n.names() {
                        set.insert(name.clone());
                    }
                }
                ColorSpace::Indexed(indexed) => {
                    resolve_colorant_name_from_cs(indexed.base(), set);
                }
                ColorSpace::Pattern(Some(colorspace)) => {
                    resolve_colorant_name_from_cs(colorspace, set);
                }
                _ => {}
            }
        }

        let mut set = HashSet::new();
        self.iter()
            .for_each(|cs| resolve_colorant_name_from_cs(cs, &mut set));
        set
    }
}
