use crate::Result;
use pdf::{
    BBox, Color, ContentWalker, ContentWalkerStep, OcConfig, Operator, PatternColor, UserSpace,
    WalkerContext, XObject,
};

/// Walker Processor
pub trait WalkerProcessor {
    /// For each operator it processes the [`pdf::ContentWalkerStep`].
    fn process(&mut self, step: &ContentWalkerStep);
}

/// Page Walker
///
/// For a given [`pdf::PdfPage`] it runs through each operator in the content stream while
/// applying the list of provided processors.
///
/// It simplifies and manages the traversal through form xobjects and tiling patterns.
///
/// By providing an [`pdf::OcConfig`] the walker respects the oc visibility of each operator in
/// order to skip the processor evaluation when the operator is not visible.
pub struct PageWalker<'a> {
    processors: Vec<&'a mut dyn WalkerProcessor>,
    oc_config: Option<OcConfig>,
}

impl<'a> Default for PageWalker<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PageWalker<'a> {
    /// Initializes a new page walker.
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            oc_config: None,
        }
    }

    /// Adds a processor that is evaluated [`WalkerProcessor::process`] for each step (abstraction
    /// over an operator incl. graphics state) on the content stream.
    pub fn add_processor(&mut self, processor: &'a mut dyn WalkerProcessor) {
        self.processors.push(processor);
    }

    /// Returns the page walker with given oc config.
    pub fn with_oc_config(mut self, config: OcConfig) -> Self {
        self.oc_config = Some(config);
        self
    }

    /// Start the page walker.
    pub fn run(&mut self, page: &pdf::PdfPage<'_>) -> Result<()> {
        let mut walker = ContentWalker::from_page(page)?;
        if let Some(config) = &self.oc_config {
            walker = walker.with_context(WalkerContext::new(Some(config.clone())));
        }
        self.walk(&mut walker)
    }

    // But I would walk 500 miles, and I would walk 500 more... Da d-da da
    fn walk(&mut self, walker: &mut ContentWalker) -> Result<()> {
        while let Some(step) = walker.next_step()? {
            if let Some(Ok(false)) = step.oc_visible() {
                continue;
            }

            // Process this step for every processor.
            self.processors.iter_mut().for_each(|p| p.process(&step));

            match step.operator() {
                Operator::PaintXObject(xobject) => {
                    match xobject {
                        Ok(XObject::Form(form)) => {
                            let mut form_walker = walker.from_form_xobject(form)?;
                            self.walk(&mut form_walker)?;
                        }
                        Err(e) => return Err(e.clone().into()),
                        _ => (),
                    };
                }
                Operator::PaintPath { fill, stroke } => {
                    let region = step.painted_bbox();
                    if *fill {
                        self.walk_tiling(
                            walker,
                            &step.graphics_state().non_stroking.color.clone()?,
                            region.clone(),
                        )?;
                    }
                    if *stroke {
                        self.walk_tiling(
                            walker,
                            &step.graphics_state().stroking.color.clone()?,
                            region.clone(),
                        )?;
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn walk_tiling(
        &mut self,
        walker: &ContentWalker,
        color: &Color,
        region: pdf::Result<BBox<UserSpace>>,
    ) -> Result<()> {
        let tiling = match color {
            Color::Pattern(PatternColor::ColoredTiling(t))
            | Color::Pattern(PatternColor::UncoloredTiling { pattern: t, .. }) => t,
            _ => return Ok(()),
        };
        let mut pattern_walker = walker.from_tiling_pattern(tiling, region)?;
        self.walk(&mut pattern_walker)
    }
}
