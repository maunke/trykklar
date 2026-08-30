//! Patterns

use crate::codec::TryFromObject;
use crate::dict::{DictKey, read_field, read_field_with_fn, read_optional_field};
use crate::error::{
    Field, FieldError, FieldExt, OptionalField, OptionalFieldExt, ResourceKind, ResultExt,
    ResultExtDeref,
};
use crate::resources::Resources;
use crate::unit::UserSpace;
use crate::{ColorSpace, Error, Matrix, Rect, Result, object_id};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::sync::Arc;

/// Pattern
///
/// ISO 32000-1:2008 8.7 Patterns
///
/// > Patterns come in two varieties:
/// >
/// > - Tiling patterns consist of a small graphical figure (called a pattern cell) that is
/// > replicated at fixed horizontal and vertical intervals to fill the area to be painted. The
/// > graphics objects to use for tiling shall be described by a content stream.
/// > - Shading patterns define a gradient fill that produces a smooth transition between colours
/// > across the area. The colour to use shall be specified as a function of position using any of a
/// > variety of methods.
#[derive(Debug, Clone)]
pub enum Pattern<'a> {
    /// Tiling Pattern
    Tiling(Arc<TilingPattern<'a>>),
    /// Shading Pattern
    Shading(Arc<ShadingPattern>),
}

impl<'a> Pattern<'a> {
    pub(crate) fn resolve(
        pattern_key: &[u8],
        resource_dicts: &[&'a Dictionary],
        doc: &'a Document,
    ) -> Result<Self> {
        for d in resource_dicts {
            if let Ok(pattern_dict_obj) = d.get_deref(b"Pattern", doc) {
                let pattern_dict = pattern_dict_obj.as_dict()?;
                if let Ok(pattern_obj_ref) = pattern_dict.get(pattern_key) {
                    let (opt_obj_id, pattern_obj) = doc.dereference(pattern_obj_ref)?;
                    let Some(obj_id) = opt_obj_id else {
                        return Err(Error::InvalidPdfObject(
                            "Pattern must be an indirect reference",
                        ));
                    };
                    match pattern_obj {
                        Object::Stream(tiling_stream) => {
                            return Ok(Pattern::Tiling(Arc::new(TilingPattern::resolve(
                                obj_id,
                                tiling_stream,
                                doc,
                            )?)));
                        }
                        Object::Dictionary(shading_dict) => {
                            return Ok(Pattern::Shading(Arc::new(ShadingPattern::resolve(
                                obj_id,
                                shading_dict,
                                resource_dicts,
                                doc,
                            )?)));
                        }
                        _ => {
                            return Err(Error::ResourceNotFound {
                                kind: ResourceKind::Pattern,
                            });
                        }
                    }
                }
            }
        }
        Err(Error::ResourceNotFound {
            kind: ResourceKind::Pattern,
        })
    }
}

object_id!(TilingPatternId);

/// Tiling Pattern
///
/// ISO 32000-1:2008 8.7.3 Tiling Patterns
///
/// > A tiling pattern consists of a small graphical figure called a pattern cell. Painting with the
/// > pattern replicates the cell at fixed horizontal and vertical intervals to fill an area. The
/// > effect is as if the figure were painted on the surface of a clear glass tile, identical copies
/// > of which were then laid down in an array covering the area and trimmed to its boundaries. This
/// > process is called tiling the area.
///
/// Table 75 – Additional Entries Specific to a Type 1 Pattern Dictionary
#[derive(Debug, Clone)]
pub struct TilingPattern<'a> {
    pub(crate) id: TilingPatternId,
    pub(crate) paint_type: Field<TilingPaintType>,
    pub(crate) resources: Field<Resources<'a>>,
    pub(crate) bbox: Field<Rect<UserSpace>>,
    pub(crate) matrix: Result<Matrix>,
    pub(crate) content: Result<Vec<u8>>,
}

impl<'a> TilingPattern<'a> {
    /// Returns the ID of tiling pattern object.
    pub fn id(&self) -> TilingPatternId {
        self.id
    }

    /// Returns the paint type.
    pub fn paint_type(&self) -> Field<TilingPaintType> {
        self.paint_type.as_field_ref().copied()
    }

    /// Returns the resources dictionary.
    ///
    /// > (Required) A resource dictionary that shall contain all of the named resources required by
    /// > the pattern’s content stream (see 7.8.3, "Resource Dictionaries").
    pub fn resources(&self) -> Field<&Resources<'a>> {
        self.resources.as_field_ref()
    }

    /// Returns the bounding box.
    ///
    /// > (Required) An array of four numbers in the pattern coordinate system giving the
    /// > coordinates of the left, bottom, right, and top edges, respectively, of the pattern cell’s
    /// > bounding box. These boundaries shall be used to clip the pattern cell.
    pub fn bbox(&self) -> Field<&Rect<UserSpace>> {
        self.bbox.as_field_ref()
    }

    /// Returns the pattern matrix.
    ///
    /// > (Optional) An array of six numbers specifying the pattern matrix (see 8.7.2, "General
    /// > Properties of Patterns").
    /// >
    /// > Default value: the identity matrix `[ 1 0 0 1 0 0 ]`.
    ///
    /// Default value implemented here: [crate::matrix::Matrix::IDENTITY]
    pub fn matrix(&self) -> Result<&Matrix> {
        self.matrix.ok_ref()
    }

    /// Returns the plain content of the tiling pattern.
    pub fn content(&self) -> Result<&[u8]> {
        self.content.ok_deref()
    }
}

impl<'a> TilingPattern<'a> {
    pub(crate) fn resolve(id: ObjectId, stream: &'a Stream, doc: &'a Document) -> Result<Self> {
        let id = TilingPatternId(id);
        let dict = &stream.dict;
        let paint_type = read_field(doc, dict);
        let resources = read_field(doc, dict);
        let bbox = read_field(doc, dict);
        let matrix = match read_optional_field(doc, dict) {
            Some(Ok(m)) => Ok(m),
            Some(Err(e)) => Err(e),
            None => Ok(Matrix::IDENTITY),
        };
        let content = stream.get_plain_content().map_err(Into::into);

        Ok(Self {
            id,
            paint_type,
            resources,
            bbox,
            matrix,
            content,
        })
    }
}

/// Paint Type
///
/// ISO 32000-1:2008 Table 75 – Additional Entries Specific to a Type 1 Pattern Dictionary
///
/// `PaintType`, integer type
///
/// > (Required) A code that determines how the colour of the pattern cell shall be specified:
/// >
/// > a) Coloured tiling pattern. The pattern’s content stream shall specify the colours used to
/// > paint the pattern cell. When the content stream begins execution, the current colour is the
/// > one that was initially in effect in the pattern’s parent content stream. This is similar to
/// > the definition of the pattern matrix; see 8.7.2, "General Properties of Patterns".
/// >
/// > b) Uncoloured tiling pattern. The pattern’s content stream shall not specify any colour
/// > information. Instead, the entire pattern cell is painted with a separately specified colour
/// > each time the pattern is used. Essentially, the content stream describes a stencil through
/// > which the current colour shall be poured. The content stream shall not invoke operators that
/// > specify colours or other colour- related parameters in the graphics state; otherwise, an error
/// > occurs (see 8.6.8, "Colour Operators"). The content stream may paint an image mask, however,
/// > since it does not specify any colour information (see 8.9.6.2, "Stencil Masking").
#[derive(Debug, Clone, Copy)]
pub enum TilingPaintType {
    /// `1`
    Coloured,
    /// `2`
    Uncoloured,
}

impl DictKey for TilingPaintType {
    const KEY: &'static [u8] = b"PaintType";
}

impl<'a> TryFromObject<'a> for TilingPaintType {
    fn try_from_object(_doc: &'a Document, _id: Option<ObjectId>, obj: &'a Object) -> Result<Self> {
        let paint_type = match obj.as_i64()? {
            1 => Self::Coloured,
            2 => Self::Uncoloured,
            _ => return Err(Error::InvalidPdfObject("Tiling paint type must be 1 or 2")),
        };
        Ok(paint_type)
    }
}

object_id!(ShadingPatternId);

/// Shading Pattern
///
/// ISO 32000-1:2008 8.7.4 Shading Patterns
///
/// > Shading patterns (PDF 1.3) provide a smooth transition between colours across an area to be
/// > painted, independent of the resolution of any particular output device and without specifying
/// > the number of steps in the colour transition. Patterns of this type shall be described by
/// > pattern dictionaries with a pattern type of 2. Table 76 shows the contents of this type of
/// > dictionary.
///
/// Table 76 – Entries in a Type 2 Pattern Dictionary
#[derive(Debug, Clone)]
pub struct ShadingPattern {
    pub(crate) id: ShadingPatternId,
    pub(crate) shading: Field<Shading>,
    pub(crate) matrix: Field<Matrix>,
}

impl<'a> ShadingPattern {
    /// Returns the ID of the shading pattern object.
    pub fn id(&self) -> ShadingPatternId {
        self.id
    }

    /// Returns the shading.
    pub fn shading(&self) -> Field<&Shading> {
        self.shading.as_field_ref()
    }

    /// Returns the pattern matrix.
    ///
    /// > (Optional) An array of six numbers specifying the pattern matrix (see 8.7.2, "General
    /// > Properties of Patterns").
    /// >
    /// > Default value: the identity matrix `[ 1 0 0 1 0 0 ]`.
    ///
    /// Default value implemented here: [crate::matrix::Matrix::IDENTITY]
    pub fn matrix(&self) -> Field<&Matrix> {
        self.matrix.as_field_ref()
    }

    fn resolve(
        id: ObjectId,
        dict: &Dictionary,
        resource_dicts: &[&'a Dictionary],
        doc: &Document,
    ) -> Result<Self> {
        let id = ShadingPatternId(id);
        let matrix = match read_optional_field(doc, dict) {
            Some(Ok(m)) => Ok(m),
            Some(Err(e)) => Err(e.into()),
            None => Ok(Matrix::IDENTITY),
        };
        let shading =
            read_field_with_fn(doc, dict, |obj| Shading::resolve(obj, resource_dicts, doc));

        Ok(Self {
            id,
            matrix,
            shading,
        })
    }
}

/// Shading
///
/// ISO 32000-1:2008 Table 78 – Entries Common to All Shading Dictionaries
#[derive(Debug, Clone)]
pub struct Shading {
    pub(crate) color_space: Field<ColorSpace>,
    pub(crate) bbox: OptionalField<Rect<UserSpace>>,
}

impl DictKey for Shading {
    const KEY: &'static [u8] = b"Shading";
}

impl<'a> Shading {
    /// Returns the shading color space.
    ///
    /// > (Required) The colour space in which colour values shall be expressed. This may be any
    /// > device, CIE-based, or special colour space except a Pattern space. See 8.7.4.4, "Colour
    /// > Space: Special Considerations" for further information.
    pub fn color_space(&self) -> Field<&ColorSpace> {
        self.color_space.as_field_ref()
    }

    /// Returns the shading bounding box.
    ///
    /// > (Optional) An array of four numbers giving the left, bottom, right, and top coordinates,
    /// > respectively, of the shading’s bounding box. The coordinates shall be interpreted in the
    /// > shading’s target coordinate space. If present, this bounding box shall be applied as a
    /// > temporary clipping boundary when the shading is painted, in addition to the current
    /// > clipping path and any other clipping boundaries in effect at that time.
    pub fn bbox(&self) -> OptionalField<&Rect<UserSpace>> {
        self.bbox.as_field_ref()
    }

    pub(crate) fn resolve(
        obj: &Object,
        resource_dicts: &[&'a Dictionary],
        doc: &Document,
    ) -> Result<Self> {
        let obj = doc.dereference(obj)?;
        let dict = match obj.1 {
            Object::Dictionary(d) => d,
            Object::Stream(s) => &s.dict,
            _ => return Err(Error::InvalidPdfObject("Shading must be a dict or stream")),
        };
        let color_space = match read_field_with_fn(doc, dict, |obj| {
            ColorSpace::parse_object(obj, resource_dicts, doc, 0)
        }) {
            // As in the spec defined, Pattern colorspace is not allowed.
            Ok(ColorSpace::Pattern(_)) => {
                Err(FieldError::Invalid(Error::InvalidColorSpaceNestedPattern))
            }
            cs => cs,
        };
        let bbox = read_optional_field(doc, dict);

        Ok(Shading { color_space, bbox })
    }
}
