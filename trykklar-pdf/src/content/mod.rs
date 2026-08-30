//! Content Stream Utilities
mod cache;
mod context;
mod operator;
mod state;
use crate::error::ResultExt;
use crate::font::{CMapEncoding, FontKind};
use crate::geometry::{BBox, CurrentPath, PathElement, Point};
use crate::ocg::Oc;
use crate::page::PdfPage;
use crate::text::{CharSpace, ShowText, TextElement, TextLeading, WordSpace};
use crate::unit::UserSpace;
use crate::xobject::{FormXObject, XObject};
use crate::{Error, Length, Matrix, Rect, Result, TilingPattern, UserUnit};
pub(crate) use cache::{ResolvedCache, WalkerCache};
pub use context::WalkerContext;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document};
pub use operator::Operator;
pub(crate) use operator::TryFromOperands;
pub use state::GraphicsState;
use std::sync::Arc;

/// Content Walker
///
/// The content walker walks through a page's content stream by traversing through all operations,
/// meaning the operator and its operands.
///
/// The walker complies with the ISO 32000-1:2008 by holding all required states to have a
/// conformant and lenient view on the content. It manages:
///
/// - [`GraphicsState`]
/// - GraphicsStack (handling [`Operator::SaveGraphicsState`] and
///   [`Operator::RestoreGraphicsState`])
/// - Internal Cache for named keys using resource dictionaries as [`crate::color::ColorSpace`],
///   [`crate::font::Font`], [`crate::xobject::XObject`], [`crate::extgstate::ExtGState`] and
///   [`crate::pattern::Pattern`]
/// - Base CTM
/// - OC Stack
/// - Current Path
/// - Current Point
#[derive(Debug, Clone)]
pub struct ContentWalker<'a> {
    doc: &'a Document,
    context: Option<Arc<WalkerContext>>,
    cache: WalkerCache<'a>,
    resource_dicts: Vec<&'a Dictionary>,
    raw_operations: Arc<Vec<Arc<Operation>>>,
    step: usize,
    graphics_state: GraphicsState<'a>,
    // Represents q/Q operator mechanism
    graphics_stack: Vec<GraphicsState<'a>>,
    // Maps the content stream's default space to page space, as for pattern.
    base_ctm: Result<Matrix>,
    // A filled region when a tiling pattern is traversed through. `None` means, the walker is not
    // withing a tiling pattern.
    tiled_region: Option<Result<BBox<UserSpace>>>,
    // Manages oc stack introduced by marked content operators.
    oc_stack: Vec<Result<Option<Result<Oc>>>>,
    current_path: CurrentPath,
    subpath_start: Option<Result<Point<UserSpace>>>,
    current_point: Option<Result<Point<UserSpace>>>,
    // Tracks the text bounding box
    text_bbox: Result<BBox<UserSpace>>,
    text_matrix: Result<Matrix>,
    line_matrix: Result<Matrix>,
    // Tracks the depth in order to prevent infinite recursion of nested walkers.
    depth: usize,
    // User Unit of the Page
    user_unit: Result<UserUnit>,
}

// Computes the bounding box of the current path.
fn compute_current_path_bbox(current_path: &CurrentPath) -> Result<BBox<UserSpace>> {
    let mut bbox = BBox::default();
    for el in current_path.iter() {
        match el {
            Ok(el) => el.for_each_point(|p| bbox.include(p.x.get(), p.y.get())),
            Err(e) => return Err(e.clone()),
        }
    }
    Ok(bbox)
}

impl<'a> ContentWalker<'a> {
    fn path_bbox(&self) -> Result<BBox<UserSpace>> {
        compute_current_path_bbox(&self.current_path)
    }
}

/// Content Walker Step
///
/// The step representates an operator the related state.
#[derive(Debug, Clone)]
pub struct ContentWalkerStep<'a> {
    raw_operation: Arc<Operation>,
    context: Option<Arc<WalkerContext>>,
    operator: Operator<'a>,
    graphics_state: GraphicsState<'a>,
    current_path: CurrentPath,
    current_point: Option<Result<Point<UserSpace>>>,
    text_bbox: Result<BBox<UserSpace>>,
    text_matrix: Result<Matrix>,
    line_matrix: Result<Matrix>,
    oc_levels: Vec<Result<Oc>>,
    tiled_region: Option<Result<BBox<UserSpace>>>,
    user_unit: Result<UserUnit>,
}

impl<'a> ContentWalkerStep<'a> {
    /// Returns the raw operation, containing the operator and operands.
    pub fn raw_operation(&self) -> &Arc<Operation> {
        &self.raw_operation
    }

    /// Returns the walker context.
    pub fn context(&self) -> Option<Arc<WalkerContext>> {
        self.context.as_ref().cloned()
    }

    /// Returns the lenient parsed operator.
    pub fn operator(&self) -> &Operator<'a> {
        &self.operator
    }

    /// Returns the graphics state.
    pub fn graphics_state(&self) -> &GraphicsState<'a> {
        &self.graphics_state
    }

    /// Returns the current point.
    pub fn current_point(&self) -> Option<&Result<Point<UserSpace>>> {
        self.current_point.as_ref()
    }

    /// Returns the current path.
    pub fn current_path(&self) -> &CurrentPath {
        &self.current_path
    }

    /// Returns the path bounding box.
    pub fn path_bbox(&self) -> Result<BBox<UserSpace>> {
        compute_current_path_bbox(&self.current_path)
    }

    /// Returns the text bounding box.
    pub fn text_bbox(&self) -> Result<&BBox<UserSpace>> {
        self.text_bbox.ok_ref()
    }

    /// Returns the text matrix.
    pub fn text_matrix(&self) -> Result<&Matrix> {
        self.text_matrix.ok_ref()
    }

    /// Returns the line matrix.
    pub fn line_matrix(&self) -> Result<&Matrix> {
        self.line_matrix.ok_ref()
    }

    /// Returns the oc levels.
    ///
    /// It originates from the oc stack of the [`ContentWalker`] by having `None` elements filtered
    /// out.
    pub fn oc_levels(&self) -> &[Result<Oc>] {
        &self.oc_levels
    }

    /// Returns the user unit of the page.
    pub fn user_unit(&self) -> Result<UserUnit> {
        self.user_unit.ok_ref().copied()
    }

    /// Whether this step walks within a tiling pattern.
    pub fn is_tiled(&self) -> bool {
        self.tiled_region.is_some()
    }

    fn clip_bbox(&self, bbox: &Result<BBox<UserSpace>>) -> Result<BBox<UserSpace>> {
        let clip = &self.graphics_state.clipping_bbox;
        let cell = match (clip, bbox) {
            (Err(e), _) | (_, Err(e)) => return Err(e.clone()),
            (Ok(c), Ok(m)) => c.intersect(m),
        };
        match &self.tiled_region {
            Some(_) if cell.is_empty() => Ok(cell),
            Some(region) => region.clone(),
            None => Ok(cell),
        }
    }

    /// Returns the bounding box, where the ink is painted on.
    ///
    /// This can be used to check for the usage of separations within a given bounding box.
    pub fn painted_bbox(&self) -> Result<BBox<UserSpace>> {
        match &self.operator {
            Operator::PaintPath { fill, stroke } if *fill || *stroke => {
                self.clip_bbox(&self.path_bbox())
            }
            Operator::ShowText(_) => match &self.graphics_state.text.mode {
                Ok(m) if !m.fills() && !m.strokes() => Ok(BBox::default()),
                Ok(_) => self.clip_bbox(&self.text_bbox),
                Err(e) => Err(e.clone()),
            },
            Operator::PaintXObject(xobject) => match xobject {
                // Image maps the unit rectangle with CTM.
                //
                // ISO 32000-1:2008 8.9.4 Image Coordinate System
                Ok(XObject::Image(_)) => match &self.graphics_state.ctm {
                    Ok(ctm) => self.clip_bbox(&Ok(ctm.transform_rect_bounds(&Rect::UNIT).into())),
                    Err(e) => Err(e.clone()),
                },
                // The default bounding box is used here, meaning it is_empty. The form xobject
                // operators are holding the painted_bbox information.
                Ok(XObject::Form(_)) => Ok(BBox::default()),
                Err(e) => Err(e.clone()),
            },
            // A shading operator paints at most the entire current clip.
            Operator::PaintShading(sh) => match sh {
                Ok(shading) => match shading.bbox() {
                    Some(Ok(bbox)) => match &self.graphics_state.ctm {
                        Ok(ctm) => self.clip_bbox(&Ok(ctm.transform_rect_bounds(bbox).into())),
                        Err(e) => Err(e.clone()),
                    },
                    Some(Err(e)) => Err(e),
                    None => self.clip_bbox(&self.graphics_state.clipping_bbox),
                },
                Err(e) => Err(e.clone()),
            },
            // Non painting operator
            _ => Ok(BBox::default()),
        }
    }

    fn all_visible(&self, xobject_oc: Option<&Option<Result<Oc>>>) -> Option<Result<bool>> {
        let d = self.context.as_ref()?.oc_default_config.as_ref()?;
        let levels = self
            .oc_levels
            .iter()
            .chain(xobject_oc.into_iter().flatten());
        for oc in levels {
            match oc {
                Ok(oc) => match d.oc_visible(oc) {
                    Ok(false) => return Some(Ok(false)),
                    Err(e) => return Some(Err(e.clone())),
                    _ => (),
                },
                Err(e) => return Some(Err(e.clone())),
            }
        }
        Some(Ok(true))
    }

    /// Returns the visibility state. `None` if no context is provided.
    pub fn oc_visible(&self) -> Option<Result<bool>> {
        let xobject_oc = match &self.operator {
            Operator::PaintXObject(xobject) => match xobject {
                Ok(XObject::Image(image)) => Some(image.oc()),
                Ok(XObject::Form(form)) => Some(form.oc()),
                Err(e) => return Some(Err(e.clone())),
            },
            _ => None,
        };
        self.all_visible(xobject_oc)
    }
}

impl<'a> ContentWalker<'a> {
    const MAX_DEPTH: usize = 128;

    /// Returns the content walker with the given context.
    pub fn with_context(mut self, context: WalkerContext) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Initializes the content walker from a page.
    pub fn from_page(page: &PdfPage<'a>) -> Result<Self> {
        let doc = page.doc();
        let id = page.id();
        let user_unit = page.user_unit();
        let page_resources = doc.get_page_resources(id.get())?;
        let mut resource_dicts = Vec::new();
        if let Some(res_dict) = page_resources.0 {
            resource_dicts.push(res_dict);
        }
        for resource_id in page_resources.1 {
            let resource_dict = doc.get_dictionary(resource_id)?;
            resource_dicts.push(resource_dict);
        }
        let content_data = doc.get_page_content(id.get());
        let content = Content::decode_strict(&content_data)?;
        let raw_operations: Arc<Vec<_>> = content
            .operations
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>()
            .into();

        let step = 0;
        let mut graphics_state = GraphicsState::default();
        // Init clip bbox with mediabox of page.
        let media_box = page.media_box_user();
        graphics_state.clipping_bbox = media_box.map(|mb| mb.get().into());
        let graphics_stack = Vec::new();
        let oc_stack = Vec::new();
        let current_path = CurrentPath::default();
        let subpath_start = None;
        let current_point = None;
        let text_bbox = Ok(BBox::default());
        let text_matrix = Ok(Matrix::IDENTITY);
        let line_matrix = Ok(Matrix::IDENTITY);
        let depth = 0;
        Ok(Self {
            doc,
            context: None,
            cache: Default::default(),
            resource_dicts,
            raw_operations,
            step,
            graphics_state,
            graphics_stack,
            base_ctm: Ok(Matrix::IDENTITY),
            tiled_region: None,
            oc_stack,
            current_path,
            subpath_start,
            current_point,
            text_bbox,
            text_matrix,
            line_matrix,
            depth,
            user_unit,
        })
    }

    /// Initializes the content walker from a form xobject.
    pub fn from_form_xobject(&self, form: &'a FormXObject<'a>) -> Result<Self> {
        let mut resource_dicts = Vec::new();
        if let Some(form_resources_result) = form.resources() {
            let form_resources = form_resources_result.ok_ref()?;
            resource_dicts.push(form_resources.get());
        };
        resource_dicts.extend_from_slice(&self.resource_dicts);
        let form_content = form.content()?;
        let content = Content::decode_strict(form_content)?;
        let raw_operations: Arc<Vec<_>> = content
            .operations
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>()
            .into();

        let mut graphics_state = self.graphics_state.clone();
        match form.matrix() {
            Ok(form_matrix) => {
                if let Ok(ctm) = &mut graphics_state.ctm {
                    *ctm = ctm.pre_concat(form_matrix);
                }
            }
            err => graphics_state.ctm = err.copied(),
        };
        let form_bbox = form.bbox().ok_ref().copied();
        let form_clip = match (graphics_state.ctm.as_ref(), form_bbox) {
            (Ok(ctm), Ok(rect)) => Ok(ctm.transform_rect_bounds(rect).into()),
            (Err(e), _) => Err(e.clone()),
            (_, Err(e)) => Err(e),
        };
        let new_clip = match (&graphics_state.clipping_bbox, form_clip) {
            (Err(e), _) => Err(e.clone()),
            (_, Err(e)) => Err(e),
            (Ok(parent), Ok(fc)) => Ok(parent.intersect(&fc)),
        };
        graphics_state.clipping_bbox = new_clip;
        let oc_stack = match form.oc() {
            Some(oc) => vec![Ok(Some(oc.clone()))],
            None => vec![],
        };
        let current_path = CurrentPath::default();
        let subpath_start = None;
        let current_point = None;
        let text_bbox = Ok(BBox::default());
        let text_matrix = Ok(Matrix::IDENTITY);
        let line_matrix = Ok(Matrix::IDENTITY);
        let depth = self.depth + 1;
        if depth > Self::MAX_DEPTH {
            return Err(Error::ContentWalkerDepthExceeded);
        }
        Ok(Self {
            doc: self.doc,
            context: self.context.clone(),
            cache: Default::default(),
            resource_dicts,
            raw_operations,
            step: 0,
            base_ctm: graphics_state.ctm.clone(),
            tiled_region: self.tiled_region.clone(),
            graphics_state,
            graphics_stack: Default::default(),
            oc_stack,
            current_path,
            subpath_start,
            current_point,
            text_bbox,
            text_matrix,
            line_matrix,
            depth,
            user_unit: self.user_unit.clone(),
        })
    }

    /// Initializes the content walker from a tiling pattern.
    ///
    /// The `region` represents the painted bounding box, where the tiling pattern is applied to.
    pub fn from_tiling_pattern(
        &self,
        tiling_pattern: &TilingPattern<'a>,
        region: Result<BBox<UserSpace>>,
    ) -> Result<Self> {
        let mut resource_dicts = Vec::new();
        let tiling_pattern_resources = tiling_pattern.resources.ok_ref()?;
        resource_dicts.push(tiling_pattern_resources.get());
        resource_dicts.extend_from_slice(&self.resource_dicts);
        let tiling_pattern_content = tiling_pattern.content.ok_ref()?;
        let content = Content::decode_strict(tiling_pattern_content)?;
        let raw_operations: Arc<Vec<_>> = content
            .operations
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>()
            .into();

        let mut graphics_state = GraphicsState::default();

        match &tiling_pattern.matrix {
            Ok(mat) => {
                graphics_state.ctm = self.base_ctm.ok_ref().map(|m| m.pre_concat(mat));
            }
            e => graphics_state.ctm = e.clone(),
        };
        let pattern_bbox = tiling_pattern.bbox.ok_ref().copied();
        graphics_state.clipping_bbox = match (graphics_state.ctm.as_ref(), pattern_bbox) {
            (Ok(ctm), Ok(rect)) => Ok(ctm.transform_rect_bounds(&rect).into()),
            (Err(e), _) => Err(e.clone()),
            (_, Err(e)) => Err(e),
        };
        let oc_stack = vec![];
        let current_path = CurrentPath::default();
        let subpath_start = None;
        let current_point = None;
        let text_bbox = Ok(BBox::default());
        let text_matrix = Ok(Matrix::IDENTITY);
        let line_matrix = Ok(Matrix::IDENTITY);
        let depth = self.depth + 1;
        if depth > Self::MAX_DEPTH {
            return Err(Error::ContentWalkerDepthExceeded);
        }
        Ok(Self {
            doc: self.doc,
            context: self.context.clone(),
            cache: Default::default(),
            resource_dicts,
            raw_operations,
            step: 0,
            base_ctm: graphics_state.ctm.clone(),
            tiled_region: Some(region),
            graphics_state,
            graphics_stack: Default::default(),
            oc_stack,
            current_path,
            subpath_start,
            current_point,
            text_bbox,
            text_matrix,
            line_matrix,
            depth,
            user_unit: self.user_unit.clone(),
        })
    }

    fn move_text(&mut self, tx: f64, ty: f64) {
        match &self.line_matrix {
            Ok(lm) => {
                let m = lm.pre_concat(&Matrix::translate(tx, ty));
                self.line_matrix = Ok(m);
                self.text_matrix = Ok(m);
            }
            Err(e) => {
                let e = e.clone();
                self.line_matrix = Err(e.clone());
                self.text_matrix = Err(e);
            }
        }
    }

    fn process_show_text(&mut self, st: &ShowText) {
        if let Some((aw, ac)) = st.spacing() {
            self.graphics_state.text.word_space = Ok(WordSpace::new(aw));
            self.graphics_state.text.char_space = Ok(CharSpace::new(ac));
        }
        if st.next_line() {
            let ty = self
                .graphics_state
                .text
                .text_leading
                .as_ref()
                .map(|tl| -tl.get())
                .map_err(Clone::clone);
            match ty {
                Ok(ty) => self.move_text(0.0, ty),
                Err(e) => {
                    self.text_matrix = Err(e.clone());
                    self.text_bbox = Err(e);
                    return;
                }
            }
        }
        match self.compute_text_bbox_and_matrix(st) {
            Ok((bbox, tm)) => {
                self.text_bbox = Ok(bbox);
                self.text_matrix = Ok(tm);
            }
            Err(e) => {
                self.text_bbox = Err(e.clone());
                self.text_matrix = Err(e);
            }
        }
    }

    /// Computes the text bounding box and text matrix.
    fn compute_text_bbox_and_matrix(
        &self,
        show_text: &ShowText,
    ) -> Result<(BBox<UserSpace>, Matrix)> {
        let gstate = &self.graphics_state;
        let font = match &gstate.text.font {
            Some(Ok(f)) => f,
            Some(Err(e)) => return Err(e.clone()),
            None => return Err(Error::InvalidPdfObject("show text with no font set")),
        };
        let tfs = match &gstate.text.font_size {
            Some(Ok(s)) => s.get(),
            Some(Err(e)) => return Err(e.clone()),
            None => return Err(Error::InvalidPdfObject("show text with no font size set")),
        };
        let ctm = *gstate.ctm.ok_ref()?;
        let tm = *self.text_matrix.ok_ref()?;
        let tc = gstate.text.char_space.ok_ref()?.get();
        let tw = gstate.text.word_space.ok_ref()?.get();
        let th = gstate.text.horizontal_space.ok_ref()?.get() / 100.0;
        let rise = gstate.text.text_rise.ok_ref()?.get();

        let font_bbox = font.font_bbox()?.rect();
        let font_matrix = *font.font_matrix()?.get();

        let param_matrix = Matrix {
            a: tfs * th,
            b: 0.0,
            c: 0.0,
            d: tfs,
            e: 0.0,
            f: rise,
        };
        let single_byte = !matches!(&**font, FontKind::Type0(_));

        if let FontKind::Type0(f) = &**font
            && matches!(f.encoding()?, CMapEncoding::IdentityV)
        {
            return Err(Error::Unsupported("vertical writing mode is not supported"));
        }

        let glyph_box = param_matrix
            .pre_concat(&font_matrix)
            .transform_rect_bounds(&font_bbox);

        let mut displacement = 0.0;
        let mut shown = false;
        let (mut min_displacement, mut max_displacement) = (0.0_f64, 0.0_f64);

        for element in show_text.elements() {
            match element {
                TextElement::Adjustment(adj) => {
                    displacement += -*adj / 1000.0 * tfs * th;
                }
                TextElement::Text(bytes) => {
                    for code in font.decode_string(bytes)? {
                        let w = font
                            .glyph_width(code)
                            .ok_or(Error::InvalidPdfObject("glyph width unavailable"))?;
                        let w_text = w * font_matrix.a;
                        let word = if single_byte && code == 32 { tw } else { 0.0 };
                        let advance = (w_text * tfs + tc + word) * th;
                        let (lo, hi) = (
                            displacement.min(displacement + advance),
                            displacement.max(displacement + advance),
                        );
                        match shown {
                            true => {
                                min_displacement = min_displacement.min(lo);
                                max_displacement = max_displacement.max(hi);
                            }
                            false => {
                                (min_displacement, max_displacement) = (lo, hi);
                                shown = true;
                            }
                        }
                        displacement += advance;
                    }
                }
            }
        }

        let bbox = match shown {
            true => {
                let bearing = glyph_box.origin.x.get();
                let run = Rect::from_edges(
                    Point {
                        x: Length::from_raw(min_displacement + bearing),
                        y: glyph_box.origin.y,
                    },
                    Point {
                        x: Length::from_raw(max_displacement + bearing.abs()),
                        y: glyph_box.origin.y + glyph_box.size.height,
                    },
                );
                ctm.pre_concat(&tm).transform_rect_bounds(&run).into()
            }
            false => BBox::default(),
        };

        Ok((bbox, tm.pre_concat(&Matrix::translate(displacement, 0.0))))
    }

    /// Traverse to the next operator in the content stream.
    pub fn next_step(&mut self) -> Result<Option<ContentWalkerStep<'a>>> {
        let Some(raw_operation) = self.raw_operations.get(self.step).cloned() else {
            return Ok(None);
        };
        self.step += 1;

        let operator = Operator::resolve(
            &raw_operation,
            &self.resource_dicts,
            self.doc,
            &mut self.cache,
        )?;

        // Graphics State Handling
        match operator.clone() {
            Operator::SaveGraphicsState => self.graphics_stack.push(self.graphics_state.clone()),
            Operator::RestoreGraphicsState => match self.graphics_stack.pop() {
                Some(gs) => self.graphics_state = gs,
                None => return Err(crate::Error::InvalidGraphicsStack),
            },
            Operator::SetStrokingColorSpace(cs, c) => {
                self.graphics_state.stroking.color_space = cs;
                self.graphics_state.stroking.color = c;
            }
            Operator::SetNonStrokingColorSpace(cs, c) => {
                self.graphics_state.non_stroking.color_space = cs;
                self.graphics_state.non_stroking.color = c;
            }
            Operator::SetStrokingColor(color) => {
                self.graphics_state.stroking.color = color;
            }
            Operator::SetNonStrokingColor(color) => {
                self.graphics_state.non_stroking.color = color;
            }
            Operator::ModifyCtm(matrix) => match matrix {
                Ok(mat) => {
                    if let Ok(m) = &mut self.graphics_state.ctm {
                        *m = m.pre_concat(&mat);
                    }
                }
                e => self.graphics_state.ctm = e.clone(),
            },
            Operator::SetTextRenderMode(mode) => self.graphics_state.text.mode = mode,
            Operator::SetFontSize { font, size } => {
                self.graphics_state.text.font_size = Some(size);
                self.graphics_state.text.font = Some(font);
            }
            Operator::SetCharSpace(char_space) => self.graphics_state.text.char_space = char_space,
            Operator::SetWordSpace(word_space) => self.graphics_state.text.word_space = word_space,
            Operator::SetHorizontalScale(horizontal_space) => {
                self.graphics_state.text.horizontal_space = horizontal_space
            }
            Operator::SetTextLeading(text_leading) => {
                self.graphics_state.text.text_leading = text_leading
            }
            Operator::SetTextRise(text_rise) => self.graphics_state.text.text_rise = text_rise,
            Operator::BeginText | Operator::EndText => {
                self.text_matrix = Ok(Matrix::IDENTITY);
                self.line_matrix = Ok(Matrix::IDENTITY);
            }
            Operator::SetTextMatrix(matrix) => {
                self.text_matrix = matrix.clone();
                self.line_matrix = matrix;
            }
            Operator::MoveText(mv) => match mv {
                Ok(m) => self.move_text(m.tx(), m.ty()),
                Err(e) => {
                    self.line_matrix = Err(e.clone());
                    self.text_matrix = Err(e);
                }
            },
            Operator::MoveTextSetLeading(mv) => match mv {
                Ok(m) => {
                    self.graphics_state.text.text_leading = Ok(TextLeading::new(-m.ty())); // TL = -ty
                    self.move_text(m.tx(), m.ty());
                }
                Err(e) => {
                    self.graphics_state.text.text_leading = Err(e.clone());
                    self.line_matrix = Err(e.clone());
                    self.text_matrix = Err(e);
                }
            },
            Operator::NextLine => match &self.graphics_state.text.text_leading {
                Ok(tl) => {
                    let ty = -tl.get();
                    self.move_text(0.0, ty);
                }
                Err(e) => {
                    self.line_matrix = Err(e.clone());
                    self.text_matrix = Err(e.clone());
                }
            },
            Operator::BeginMarkedContent { oc } => self.oc_stack.push(oc),
            Operator::EndMarkedContent if self.oc_stack.pop().is_none() => {
                return Err(Error::InvalidOcStack);
            }
            Operator::SetLineWidth(width) => self.graphics_state.line_width = width,
            Operator::SetExtGState(extgstate) => match &*extgstate {
                Ok(state) => state.clone().apply_to(&mut self.graphics_state),
                Err(e) => self.graphics_state.handle_ext_g_state_error(e),
            },
            Operator::ShowText(show) => match show {
                Ok(st) => self.process_show_text(&st),
                Err(e) => {
                    self.text_bbox = Err(e.clone());
                    self.text_matrix = Err(e);
                }
            },
            Operator::ConstructPath(path_element_result) => {
                let path_element_result = match (&self.graphics_state.ctm, path_element_result) {
                    (Ok(ctm), Ok(element)) => Ok(element.transform(ctm)),
                    (Err(e), _) => Err(e.clone()),
                    (_, Err(e)) => Err(e),
                };
                self.current_path.push(path_element_result.clone());
                match path_element_result {
                    Ok(path_element) => match path_element {
                        PathElement::MoveTo(move_to) => {
                            let p = move_to.get();
                            self.current_point = Some(Ok(p));
                            self.subpath_start = Some(Ok(p));
                        }
                        PathElement::LineTo(line_to) => {
                            self.current_point = Some(Ok(line_to.get()));
                        }
                        PathElement::CurveTo(curve_to) => {
                            self.current_point = Some(Ok(curve_to.get().2));
                        }
                        PathElement::CurveToControlCurrentTwo(curve) => {
                            self.current_point = Some(Ok(curve.get().1));
                        }
                        PathElement::CurveToControlOneThree(curve) => {
                            self.current_point = Some(Ok(curve.get().1));
                        }
                        PathElement::Rect(r) => {
                            let p = r.origin;
                            self.subpath_start = Some(Ok(p));
                            self.current_point = Some(Ok(p));
                        }
                        PathElement::Close => self.current_point = self.subpath_start.clone(),
                    },
                    Err(e) => self.current_point = Some(Err(e)),
                }
            }
            Operator::IntersectClip { .. } if !self.current_path.is_empty() => {
                let new_clip = match (&self.graphics_state.clipping_bbox, &self.path_bbox()) {
                    (Err(e), _) | (_, Err(e)) => Err(e.clone()),
                    (Ok(c), Ok(path)) => Ok(c.intersect(path)),
                };
                self.graphics_state.clipping_bbox = new_clip;
            }
            _ => (),
        }

        let is_painting = matches!(operator, Operator::PaintPath { .. });

        // Filter `None` values from oc stack to have meaningful oc levels.
        let oc_levels = self
            .oc_stack
            .iter()
            .filter_map(|oc| match oc {
                Ok(None) => None,
                Ok(Some(oc)) => Some(oc.clone()),
                Err(e) => Some(Err(e.clone())),
            })
            .collect();

        let walker_step = ContentWalkerStep {
            raw_operation: raw_operation.clone(),
            context: self.context.clone(),
            operator,
            graphics_state: self.graphics_state.clone(),
            oc_levels,
            text_bbox: self.text_bbox.clone(),
            current_path: self.current_path.clone(),
            current_point: self.current_point.clone(),
            text_matrix: self.text_matrix.clone(),
            line_matrix: self.line_matrix.clone(),
            tiled_region: self.tiled_region.clone(),
            user_unit: self.user_unit.clone(),
        };

        if is_painting {
            self.current_path = CurrentPath::default();
            self.subpath_start = None;
            self.current_point = None;
        };

        Ok(Some(walker_step))
    }
}
