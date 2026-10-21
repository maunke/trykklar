//! External Objects
use crate::codec::TryFromObject;
use crate::color::{ColorSpace, IccBased, IccProfileColorSpace};
use crate::content::write_operator::WriteOperations;
use crate::dict::{self, DictKey, read_field, read_optional_field, read_optional_field_with_fn};
use crate::error::{
    Field, FieldError, FieldExt, OptionalField, OptionalFieldExt, ResourceKind, ResultExt,
    ResultExtDeref,
};
use crate::geometry::Rect;
use crate::optional_content::Oc;
use crate::page::PdfPageId;
use crate::resources::{
    ContentResourceId, ContentResources, ContentResourcesMut, Resources, ResourcesMut,
};
use crate::stream::{FilterName, StreamFilter};
use crate::unit::UserSpace;
use crate::{Error, Matrix, Pdf, Result, object_id};
use hayro_jpeg2000::ColorSpace as JpxColorSpace;
use lopdf::content::Content;
use lopdf::{Dictionary, Object, ObjectId, Stream};
use std::num::NonZeroU32;
use std::sync::Arc;

/// External Object
///
/// ISO 32000-1:2008 8.8 External Objects
///
/// > An external object (commonly called an XObject) is a graphics object whose contents are
/// > defined by a self-contained stream, separate from the content stream in which it is used.
/// > There are three types of external objects:
/// >
/// > - An image XObject (8.9.5, "Image Dictionaries") represents a sampled visual image such as a
/// > photograph.
/// > - A form XObject (8.10, "Form XObjects") is a self-contained description of an arbitrary
/// > sequence of graphics objects.
/// > - A PostScript XObject (8.8.2, "PostScript XObjects") contains a fragment of code expressed in
/// > the PostScript page description language. PostScript XObjects should not be used.
///
/// PostScript XObjects are unsupported.
#[derive(Debug, Clone)]
pub enum XObject<'a> {
    /// Form XObject
    Form(Arc<FormXObject<'a>>),
    /// Image XObject
    Image(Arc<ImageXObject>),
}

impl<'a> XObject<'a> {
    pub(crate) fn resolve(
        xobject_key: &[u8],
        content_resources: &ContentResources<'a>,
        pdf: &'a Pdf,
    ) -> Result<Self> {
        for d in content_resources.resource_dictionaries() {
            if let Ok(xobject_dict_obj) = d.get_deref(b"XObject", pdf.doc()) {
                let xobject_dict = xobject_dict_obj.as_dict()?;
                if let Ok(xobject_entry) = xobject_dict.get(xobject_key) {
                    let stream_deref = pdf.doc().dereference(xobject_entry)?;
                    let stream = stream_deref.1.as_stream()?;
                    let Some(stream_id) = stream_deref.0 else {
                        return Err(Error::ResourceNotFound {
                            kind: ResourceKind::XObject,
                        });
                    };
                    let subtype = stream.dict.get(b"Subtype")?.as_name()?;
                    return Ok(match subtype {
                        b"Form" => {
                            XObject::Form(Arc::new(FormXObject::resolve(stream_id, stream, pdf)?))
                        }
                        b"Image" => XObject::Image(Arc::new(ImageXObject::resolve(
                            stream_id,
                            stream,
                            content_resources,
                            pdf,
                        )?)),
                        _ => {
                            return Err(Error::ResourceNotFound {
                                kind: ResourceKind::XObject,
                            });
                        }
                    });
                }
            }
        }
        Err(Error::ResourceNotFound {
            kind: ResourceKind::XObject,
        })
    }
}

object_id!(FormXObjectId);

/// Form XObject
///
/// ISO 32000-1:2008 8.10 Form XObjects
///
/// > A form XObject is a PDF content stream that is a self-contained description of any sequence of
/// > graphics objects (including path objects, text objects, and sampled images). A form XObject
/// > may be painted multiple times—either on several pages or at several locations on the same
/// > page—and produces the same results each time, subject only to the graphics state at the time
/// > it is invoked. Not only is this shared definition economical to represent in the PDF file, but
/// > under suitable circumstances the conforming reader can optimize execution by caching the
/// > results of rendering the form XObject for repeated reuse.
///
/// This will be very useful for imposition or to merge several PDFs into one.
///
/// ISO 32000-1:2008 8.10.2 Table 95 – Additional Entries Specific to a Type 1 Form Dictionary
#[derive(Debug, Clone)]
pub struct FormXObject<'a> {
    id: FormXObjectId,
    oc: OptionalField<Oc>,
    bbox: Field<Rect<UserSpace>>,
    matrix: Result<Matrix>,
    resources: OptionalField<Resources<'a>>,
    content: Result<Vec<u8>>,
    group: OptionalField<GroupXObject>,
}

impl<'a> FormXObject<'a> {
    /// Returns the ID of the form xobject object.
    pub fn id(&self) -> FormXObjectId {
        self.id
    }

    /// Returns the optional content.
    ///
    /// > (Optional; PDF 1.5) An optional content group or optional content membership dictionary
    /// > (see 8.11, "Optional Content") specifying the optional content properties for the form
    /// > XObject. Before the form is processed, its visibility shall be determined based on this
    /// > entry. If it is determined to be invisible, the entire form shall be skipped, as if there
    /// > were no Do operator to invoke it.
    pub fn oc(&self) -> &OptionalField<Oc> {
        &self.oc
    }

    /// Returns the bounding box.
    pub fn bbox(&self) -> Field<&Rect<UserSpace>> {
        self.bbox.as_field_ref()
    }

    /// Returns the form matrix.
    ///
    /// > (Optional) An array of six numbers specifying the form matrix, which maps form space into
    /// > user space (see 8.3.4, "Transformation Matrices").
    /// >
    /// > Default value: the identity matrix `[ 1 0 0 1 0 0 ]`.
    pub fn matrix(&self) -> Result<&Matrix> {
        self.matrix.ok_ref()
    }

    /// The content of the form xobject.
    pub fn content(&self) -> Result<&[u8]> {
        self.content.ok_deref()
    }

    /// Returns the [`GroupXObject`].
    pub fn group(&self) -> OptionalField<&GroupXObject> {
        self.group.as_field_ref()
    }

    /// Returns the resources dictionary.
    ///
    /// > (Optional but strongly recommended; PDF 1.2) A dictionary specifying any resources (such
    /// > as fonts and images) required by the form XObject (see 7.8, "Content Streams and
    /// > Resources").
    /// >
    /// > In a PDF whose version is 1.1 and earlier, all named resources used in the
    /// > form XObject shall be included in the resource dictionary of each page object on which the
    /// > form XObject appears, regardless of whether they also appear in the resource dictionary of
    /// > the form XObject. These resources should also be specified in the form XObject’s resource
    /// > dictionary as well, to determine which resources are used inside the form XObject. If a
    /// > resource is included in both dictionaries, it shall have the same name in both locations.
    /// >
    /// > In PDF 1.2 and later versions, form XObjects may be independent of the content streams in
    /// > which they appear, and this is strongly recommended although not required. In an
    /// > independent form XObject, the resource dictionary of the form XObject is required and
    /// > shall contain all named resources used by the form XObject. These resources shall not be
    /// > promoted to the outer content stream’s resource dictionary, although that stream’s
    /// > resource dictionary refers to the form XObject.
    pub fn resources(&self) -> OptionalField<&Resources<'a>> {
        self.resources.as_field_ref()
    }

    pub(crate) fn resolve(id: ObjectId, stream: &'a Stream, pdf: &'a Pdf) -> Result<Self> {
        let id = FormXObjectId(id);
        let dict = &stream.dict;
        let oc = read_optional_field_with_fn(pdf, dict, |obj| {
            Oc::resolve(obj, &ContentResources::Empty, pdf)
        });
        let bbox = read_field(pdf, dict);
        let matrix = match read_optional_field(pdf, dict) {
            Some(Ok(m)) => Ok(m),
            Some(Err(e)) => Err(e),
            None => Ok(Matrix::IDENTITY),
        };
        let resources = read_optional_field(pdf, dict);
        let content = stream.get_plain_content().map_err(Into::into);
        let group = read_optional_field(pdf, dict);

        Ok(FormXObject {
            id,
            oc,
            bbox,
            matrix,
            resources,
            content,
            group,
        })
    }
}

/// Group XObject
///
/// ISO 32000-1:2008 Table 95 – Additional Entries Specific to a Type 1 Form Dictionary
///
/// > (Optional; PDF 1.4) A group attributes dictionary indicating that the contents of the form
/// > XObject shall be treated as a group and specifying the attributes of that group (see 8.10.3,
/// > "Group XObjects"). If a Ref entry (see below) is present, the group attributes shall also
/// > apply to the external page imported by that entry, which allows such an imported page to be
/// > treated as a group without further modification.
///
/// 8.10.3 Group XObjects
///
/// > As shown in Table 96, every group XObject shall have a group subtype (specified by the S entry
/// > in the group attributes dictionary) that determines the format and meaning of the dictionary’s
/// > remaining entries. This specification only defines one subtype, a transparency group XObject
/// > (subtype Transparency) representing a transparency group for use in the transparent imaging
/// > model (see 11.4, "Transparency Groups"). The remaining contents of this type of dictionary are
/// > described in 11.6.6, "Transparency Group XObjects".
///
/// Since only the subtype transparency group is valid, the GroupXObject will cover all transparency
/// group fields. So we do not introduce a one member enum here.
///
/// 11.6.6 Transparency Group XObjects Table 147 – Additional entries specific to a transparency
/// group attributes dictionary
#[derive(Debug, Clone)]
pub struct GroupXObject {
    sub_type: Field<GroupSubType>,
    colorspace: OptionalField<GroupColorSpace>,
    isolation: Result<GroupIsolation>,
    knockout: Result<GroupKnockout>,
}

impl GroupXObject {
    /// Returns [`GroupSubType`]
    pub fn sub_type(&self) -> Field<GroupSubType> {
        self.sub_type.as_field_ref().copied()
    }

    /// Returns [`GroupColorSpace`]
    pub fn colorspace(&self) -> OptionalField<&GroupColorSpace> {
        self.colorspace.as_field_ref()
    }

    /// Returns [`GroupIsolation`]
    pub fn isolation(&self) -> Result<GroupIsolation> {
        self.isolation.ok_ref().copied()
    }

    /// Returns [`GroupKnockout`]
    pub fn knockout(&self) -> Result<GroupKnockout> {
        self.knockout.ok_ref().copied()
    }
}

impl DictKey for GroupXObject {
    const KEY: &'static [u8] = b"Group";
}

impl TryFromObject<'_> for GroupXObject {
    fn try_from_object(pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let dict = obj.as_dict()?;
        let sub_type = read_field(pdf, dict);
        let colorspace = read_optional_field(pdf, dict);
        let isolation = match read_optional_field(pdf, dict) {
            Some(i) => i,
            None => Ok(GroupIsolation::default()),
        };
        let knockout = match read_optional_field(pdf, dict) {
            Some(i) => i,
            None => Ok(GroupKnockout::default()),
        };

        Ok(Self {
            sub_type,
            colorspace,
            isolation,
            knockout,
        })
    }
}

/// `/S` Group Subtype
///
/// ISO 32000-1:2008 Table 96 Entries Common to all Group Attributes Dictionaries
///
/// > (Required) The group subtype, which identifies the type of group whose attributes this
/// > dictionary describes and determines the format and meaning of the dictionary’s remaining
/// > entries. The only group subtype defined is Transparency; see 11.6.6, "Transparency Group
/// > XObjects", for the remaining contents of this type of dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSubType {
    /// Transparency Group XObject
    Transparency,
}

impl DictKey for GroupSubType {
    const KEY: &'static [u8] = b"S";
}

impl TryFromObject<'_> for GroupSubType {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ Object,
    ) -> Result<Self> {
        let variant = match obj.as_name()? {
            b"Transparency" => Self::Transparency,
            _ => {
                return Err(Error::InvalidPdfObject("GroupSubType must be Transparency"));
            }
        };
        Ok(variant)
    }
}

/// `/I` Group Isolation
///
/// ISO 32000-1:2008 Table 96 Entries Common to all Group Attributes Dictionaries
///
/// > (Optional) A flag specifying whether the transparency group is isolated (see “Isolated
/// > Groups”). If this flag is true, objects within the group shall be composited against a fully
/// > transparent initial backdrop; if false, they shall be composited against the group’s backdrop.
/// > Default value: false
#[derive(Debug, Clone, Copy, Default)]
pub struct GroupIsolation(bool);

impl GroupIsolation {
    /// Returns the flag boolean.
    pub fn get(&self) -> bool {
        self.0
    }
}

impl DictKey for GroupIsolation {
    const KEY: &'static [u8] = b"I";
}

impl TryFromObject<'_> for GroupIsolation {
    fn try_from_object(_pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let flag = obj.as_bool()?;
        Ok(Self(flag))
    }
}

/// `/K` Group Knockout
///
/// ISO 32000-1:2008 Table 96 Entries Common to all Group Attributes Dictionaries
///
/// > (Optional) A flag specifying whether the transparency group is a knockout group (see “Knockout
/// > Groups”). If this flag is false, later objects within the group shall be composited with
/// > earlier ones with which they overlap; if true, they shall be composited with the group’s
/// > initial backdrop and shall overwrite (“knock out”) any earlier overlapping objects. Default
/// > value: false.
#[derive(Debug, Clone, Copy, Default)]
pub struct GroupKnockout(bool);

impl GroupKnockout {
    /// Returns the flag boolean.
    pub fn get(&self) -> bool {
        self.0
    }
}

impl DictKey for GroupKnockout {
    const KEY: &'static [u8] = b"K";
}

impl TryFromObject<'_> for GroupKnockout {
    fn try_from_object(_pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let flag = obj.as_bool()?;
        Ok(Self(flag))
    }
}

/// `/CS` Group ColorSpace
///
/// ISO 32000-1:2008 Table 147 – Additional entries specific to a transparency group attributes
/// dictionary
///
/// > (Sometimes required) The group colour space, which is used for the
/// > following purposes:
/// > - As the colour space into which colours shal be converted when painted into the group
/// > - As the blending colour space in which objects shal be composited within the group (see
/// > “Blending Colour Space”)
/// > - As the colour space of the group as a whole when it in turn is painted as an object onto its
/// > backdrop
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupColorSpace(ColorSpace);

impl GroupColorSpace {
    /// Returns the [`ColorSpace`].
    pub fn get(&self) -> &ColorSpace {
        &self.0
    }
}

impl DictKey for GroupColorSpace {
    const KEY: &'static [u8] = b"CS";
}

impl TryFromObject<'_> for GroupColorSpace {
    fn try_from_object(pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let cs = ColorSpace::parse_object(obj, &ContentResources::Empty, pdf, 0);
        if matches!(
            cs,
            Ok(ColorSpace::Lab
                | ColorSpace::Pattern(..)
                | ColorSpace::DeviceN(..)
                | ColorSpace::Separation(..)
                | ColorSpace::Indexed(..))
        ) {
            return Err(Error::InvalidPdfObject(
                "provided group colorspace is not allowed",
            ));
        }
        if let Ok(ColorSpace::IccBased(ref icc_based)) = cs
            && !matches!(
                icc_based.colorspace()?,
                IccProfileColorSpace::Cmyk | IccProfileColorSpace::Gray | IccProfileColorSpace::Rgb
            )
        {
            return Err(Error::InvalidPdfObject(
                "provided group colorspace icc with non gray, rgb and cmyk is not allowed",
            ));
        }
        Ok(Self(cs?))
    }
}

/// Mutable Form XObject
pub struct FormXObjectMut<'a> {
    pdf: &'a mut Pdf,
    id: FormXObjectId,
}

impl<'a> FormXObjectMut<'a> {
    /// Creates the mutation object for a given [`FormXObjectId`].
    pub fn new(pdf: &'a mut Pdf, id: FormXObjectId) -> Self {
        Self { pdf, id }
    }

    /// Maps into the mutable resources.
    pub(crate) fn into_resources_mut(self) -> Result<ResourcesMut<'a>> {
        Ok(self
            .pdf
            .doc_mut()
            .get_object_mut(self.id.into())?
            .as_stream_mut()?
            .dict
            .get_mut(b"Resources")?
            .as_dict_mut()?
            .into())
    }

    /// Sets the content with given operations. The concept behind providing the [`PdfPageId`] is to
    /// change the resource dictionary of the page and not the form xobject in order to have a clear
    /// updated method instead of potential duplicated new entries accross nested form xobjects.
    pub fn set_content(&mut self, operations: WriteOperations, page_id: PdfPageId) -> Result<()> {
        let id = if self
            .pdf
            .doc()
            .get_object(self.id.into())?
            .as_stream()?
            .dict
            .has(b"Resources")
        {
            ContentResourceId::Form(self.id)
        } else {
            ContentResourceId::Page(page_id)
        };
        let number = self.pdf.page_number_by_id(page_id)?;
        let mut page_mut = self.pdf.page_mut(number)?;
        let mut resources = ContentResourcesMut::new(&mut page_mut, id);
        let content = Content {
            operations: operations
                .into_iter()
                .map(|op| op.try_into_operation(&mut resources))
                .collect::<Result<Vec<_>>>()?,
        };
        self.pdf
            .doc_mut()
            .change_content_stream(self.id.into(), content.encode()?);
        Ok(())
    }
}

object_id!(SoftMaskImageId);

/// Soft Mask Image
///
/// `SMask` in Table 89
///
/// > (Optional; PDF 1.4) A subsidiary image XObject defining a soft- mask image (see 11.6.5.3,
/// > "Soft-Mask Images") that shall be used as a source of mask shape or mask opacity values in the
/// > transparent imaging model. The alpha source parameter in the graphics state determines whether
/// > the mask values shall be interpreted as shape or opacity. If present, this entry shall
/// > override the current soft mask in the graphics state, as well as the image’s Mask entry, if
/// > any. However, the other transparency-related graphics state parameters—blend mode and alpha
/// > constant—shall remain in effect. If SMask is absent, the image shall have no associated soft
/// > mask (although the current soft mask in the graphics state may still apply).
///
/// ISO 32000-1:2008 11.6.5.3 Soft-Mask Images
#[derive(Debug, Clone)]
pub struct SoftMaskImageXObject {
    id: SoftMaskImageId,
    width: Field<ImageWidth>,
    height: Field<ImageHeight>,
    filter: Field<StreamFilter>,
}

impl SoftMaskImageXObject {
    /// Returns the ID.
    pub fn id(&self) -> SoftMaskImageId {
        self.id
    }

    /// Returns the image width.
    pub fn width(&self) -> Field<ImageWidth> {
        self.width.as_field_ref().copied()
    }

    /// Returns the image height.
    pub fn height(&self) -> Field<ImageHeight> {
        self.height.as_field_ref().copied()
    }

    /// Returns the image stream filter.
    pub fn filter(&self) -> Field<&StreamFilter> {
        self.filter.as_field_ref()
    }

    fn resolve(obj: &Object, pdf: &Pdf) -> Result<Self> {
        let Ok((Some(id), Object::Stream(stream))) = pdf.doc().dereference(obj) else {
            return Err(Error::ResourceNotFound {
                kind: ResourceKind::SoftMaskImage,
            });
        };
        let id = SoftMaskImageId(id);
        let dict = &stream.dict;

        let width = read_field(pdf, dict);
        let height = read_field(pdf, dict);

        let filter = match read_optional_field(pdf, dict) {
            Some(Ok(f)) => Ok(f),
            Some(Err(e)) => Err(e.into()),
            None => Ok(StreamFilter(Vec::new())),
        };

        let is_mask = dict
            .get_deref(b"ImageMask", pdf.doc())
            .and_then(Object::as_bool)
            .unwrap_or(false);
        if is_mask {
            return Err(Error::InvalidPdfObject(
                "soft mask must not be an image mask",
            ));
        }
        if dict.has(b"SMask") || dict.has(b"Mask") {
            return Err(Error::InvalidPdfObject("soft mask must not carry a mask"));
        }
        match dict.get(b"ColorSpace") {
            Ok(cs) => {
                let cs = ColorSpace::parse_object(cs, &ContentResources::Empty, pdf, 0)?;
                if cs != ColorSpace::DeviceGray {
                    return Err(Error::InvalidPdfObject("soft mask must be DeviceGray"));
                }
            }
            Err(_)
                if matches!(
                    filter.as_ref().map(|f| f.get_last()),
                    Ok(Some(Ok(FilterName::JPXDecode)))
                ) => {}
            Err(_) => return Err(Error::UndefinedColorSpace),
        }

        Ok(Self {
            id,
            width,
            height,
            filter,
        })
    }
}

object_id!(ImageId);

/// Image XObject
///
/// ISO 32000-1:2008 Table 89 – Additional Entries Specific to an Image Dictionary
#[derive(Debug, Clone)]
pub struct ImageXObject {
    id: ImageId,
    oc: OptionalField<Oc>,
    width: Field<ImageWidth>,
    height: Field<ImageHeight>,
    filter: Field<StreamFilter>,
    kind: Field<ImageKind>,
    bits_per_component: Field<BitsPerComponent>,
    soft_mask: Option<SoftMaskImageXObject>,
}

impl ImageXObject {
    /// Returns the ID.
    ///
    /// It relates to the object id in the document, NOT to the ID field in the image dict.
    pub fn id(&self) -> ImageId {
        self.id
    }

    /// Returns the optional content.
    ///
    /// > (Optional; PDF 1.5) An optional content group or optional content membership dictionary
    /// > (see 8.11, "Optional Content"), specifying the optional content properties for this image
    /// > XObject. Before the image is processed by a conforming reader, its visibility shall be
    /// > determined based on this entry. If it is determined to be invisible, the entire image
    /// > shall be skipped, as if there were no Do operator to invoke it.
    pub fn oc(&self) -> &OptionalField<Oc> {
        &self.oc
    }

    /// Returns the image width.
    pub fn width(&self) -> Field<ImageWidth> {
        self.width.as_field_ref().copied()
    }

    /// Returns the image height.
    pub fn height(&self) -> Field<ImageHeight> {
        self.height.as_field_ref().copied()
    }

    /// Returns the image stream filter.
    pub fn filter(&self) -> Field<&StreamFilter> {
        self.filter.as_field_ref()
    }

    /// Returns the image kind: Mask or Sampled that contains the colorspace.
    pub fn kind(&self) -> Field<&ImageKind> {
        self.kind.as_field_ref()
    }

    /// Returns the bits per component.
    pub fn bits_per_component(&self) -> Field<BitsPerComponent> {
        self.bits_per_component.as_field_ref().copied()
    }

    /// Returns the soft mask image.
    pub fn soft_mask(&self) -> Option<&SoftMaskImageXObject> {
        self.soft_mask.as_ref()
    }

    fn resolve(
        id: ObjectId,
        stream: &Stream,
        content_resources: &ContentResources,
        pdf: &Pdf,
    ) -> Result<Self> {
        let id = ImageId(id);
        let dict = &stream.dict;
        let oc = read_optional_field_with_fn(pdf, dict, |obj| {
            Oc::resolve(obj, &ContentResources::Empty, pdf)
        });
        let width = read_field(pdf, dict);
        let height = read_field(pdf, dict);

        let is_mask = dict
            .get_deref(b"ImageMask", pdf.doc())
            .and_then(Object::as_bool)
            .unwrap_or(false);

        let filter = match read_optional_field(pdf, dict) {
            Some(Ok(f)) => Ok(f),
            Some(Err(e)) => Err(e.into()),
            None => Ok(StreamFilter(Vec::new())),
        };
        let jpx_info = match filter.as_ref().map(|f| f.get()) {
            // Only single filter jpx is supported
            Ok([Ok(FilterName::JPXDecode)]) => Some(jpx_info(id, stream)?),
            _ => None,
        };
        let kind = if is_mask {
            Ok(ImageKind::Mask)
        } else {
            let colorspace = match dict.get(b"ColorSpace") {
                Ok(cs) => ColorSpace::parse_object(cs, content_resources, pdf, 0),
                Err(_) if let Some(ref info) = jpx_info => Ok(info.colorspace.clone()),
                Err(_) => Err(Error::UndefinedColorSpace),
            };
            colorspace
                .map(|cs| ImageKind::Sampled { colorspace: cs })
                .map_err(Into::into)
        };

        let smask_obj = match dict.get(b"SMask") {
            Ok(obj) => match pdf.doc().dereference(obj)?.1 {
                Object::Name(name) if name == b"None" => None,
                _ => Some(obj),
            },
            Err(_) => None,
        };
        let soft_mask = match (is_mask, smask_obj) {
            (true, Some(_)) => {
                return Err(Error::InvalidPdfObject(
                    "image mask must not carry a soft mask",
                ));
            }
            (false, Some(obj)) => Some(SoftMaskImageXObject::resolve(obj, pdf)?),
            _ => None,
        };

        let bits_per_component = BitsPerComponent::resolve(pdf, dict, is_mask, jpx_info);

        Ok(ImageXObject {
            id,
            oc,
            width,
            height,
            filter,
            kind,
            bits_per_component,
            soft_mask,
        })
    }
}

/// Mutable Image XObject
pub struct ImageXObjectMut<'a> {
    pdf: &'a mut Pdf,
    id: ImageId,
}

impl<'a> ImageXObjectMut<'a> {
    /// Creates the mutation object for a given [`ImageId`].
    pub fn new(pdf: &'a mut Pdf, id: ImageId) -> Self {
        Self { pdf, id }
    }

    fn dict_mut(&mut self) -> Result<&mut Dictionary> {
        Ok(&mut self
            .pdf
            .doc_mut()
            .get_object_mut(self.id.into())?
            .as_stream_mut()?
            .dict)
    }

    /// Removes the [`Oc`] entry.
    pub fn remove_oc(&mut self) -> Result<()> {
        dict::remove::<Oc>(self.dict_mut()?)
    }
}

/// Represents the kind of the image containing the colorspace in case of being a sampled image.
///
/// It relates to the `ColorSpace` key in ISO 32000-1:2008 Table 89
///
/// > (Required for images, except those that use the JPXDecode filter; not allowed forbidden for
/// > image masks) The colour space in which image samples shall be specified; it can be any type of
/// > colour space except Pattern. If the image uses the JPXDecode filter, this entry may be
/// > present:
/// > - If ColorSpace is present, any colour space specifications in the JPEG2000 data
/// > shall be ignored.
/// > - If ColorSpace is absent, the colour space specifications in the JPEG2000 data shall be used.
/// > The Decode array shall also be ignored unless ImageMask is true.
#[derive(Debug, Clone)]
pub enum ImageKind {
    /// Mask Image
    Mask,
    /// Sampled image
    Sampled {
        /// Colorspace
        colorspace: ColorSpace,
    },
}

/// Bits Per Component
///
/// `BitsPerComponent`, integer type
///
/// ISO 3200-1:2008 Table 89 – Additional Entries Specific to an Image Dictionary
///
/// > (Required except for image masks and images that use the JPXDecode filter) The number of bits
/// > used to represent each colour component. Only a single value shall be specified; the number of
/// > bits shall be the same for all colour components. The value shall be 1, 2, 4, 8, or (in PDF
/// > 1.5) 16. If ImageMask is true, this entry is optional, but if specified, its value shall be 1.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct BitsPerComponent(u8);

impl BitsPerComponent {
    fn resolve(
        pdf: &Pdf,
        dict: &Dictionary,
        is_mask: bool,
        jpx_info: Option<JpxInfo>,
    ) -> Field<Self> {
        let bits_per_component = if is_mask {
            if let Ok(obj) = dict.get(b"BitsPerComponent")
                && pdf
                    .doc()
                    .dereference(obj)
                    .map_err(|_| FieldError::Missing)?
                    .1
                    .as_i64()?
                    != 1
            {
                return Err(Into::into(Error::InvalidPdfObject(
                    "image mask must have 1 bit per component",
                )));
            }
            1
        } else if let Some(info) = &jpx_info {
            info.bit_depth
        } else {
            let obj = dict.get(b"BitsPerComponent")?;
            let value = pdf
                .doc()
                .dereference(obj)
                .map_err(|_| FieldError::Missing)?
                .1
                .as_i64()?;
            match value {
                1 | 2 | 4 | 8 | 16 => value as u8,
                _ => {
                    return Err(Into::into(Error::InvalidPdfObject(
                        "invalid bits per component",
                    )));
                }
            }
        };
        Ok(Self(bits_per_component))
    }
}

struct JpxInfo {
    colorspace: ColorSpace,
    bit_depth: u8,
}

fn jpx_info(id: ImageId, stream: &Stream) -> Result<JpxInfo> {
    let img =
        hayro_jpeg2000::Image::new(&stream.content, &hayro_jpeg2000::DecodeSettings::default())
            .map_err(|_| Error::InvalidColorSpace)?;
    let colorspace = match img.color_space() {
        JpxColorSpace::Gray => ColorSpace::DeviceGray,
        JpxColorSpace::RGB => ColorSpace::DeviceRgb,
        JpxColorSpace::CMYK => ColorSpace::DeviceCmyk,
        JpxColorSpace::Icc {
            num_channels,
            profile,
        } => ColorSpace::IccBased(Arc::new(IccBased::try_from_jpx(
            id,
            profile,
            *num_channels,
        )?)),
        JpxColorSpace::Unknown { .. } => return Err(Error::InvalidColorSpace),
    };
    let bit_depth = img.original_bit_depth();
    Ok(JpxInfo {
        colorspace,
        bit_depth,
    })
}

/// Image Width
///
/// ISO 32000-1:2008 Table 89 – Additional Entries Specific to an Image Dictionary
///
/// > (Required) The width of the image, in samples.
#[derive(Debug, Clone, Copy)]
pub struct ImageWidth(NonZeroU32);

impl ImageWidth {
    /// Returns the image width in samples.
    pub fn get(&self) -> NonZeroU32 {
        self.0
    }
}

impl DictKey for ImageWidth {
    const KEY: &'static [u8] = b"Width";
}

impl TryFromObject<'_> for ImageWidth {
    fn try_from_object(_pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let val = obj.as_i64()?;
        let width = u32::try_from(val)
            .map_err(|_| Error::InvalidPdfObject("image width must be a positive integer"))?;
        let w = width
            .try_into()
            .map_err(|_| Error::InvalidPdfObject("image width must be a positive integer > 0"))?;
        Ok(Self(w))
    }
}

/// Image Height
///
/// ISO 32000-1:2008 Table 89 – Additional Entries Specific to an Image Dictionary
///
/// > (Required) The height of the image, in samples.
#[derive(Debug, Clone, Copy)]
pub struct ImageHeight(NonZeroU32);

impl ImageHeight {
    /// Returns the image height in samples.
    pub fn get(&self) -> NonZeroU32 {
        self.0
    }
}

impl DictKey for ImageHeight {
    const KEY: &'static [u8] = b"Height";
}

impl TryFromObject<'_> for ImageHeight {
    fn try_from_object(_pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let val = obj.as_i64()?;
        let width = u32::try_from(val)
            .map_err(|_| Error::InvalidPdfObject("image height must be a positive integer"))?;
        let w = width
            .try_into()
            .map_err(|_| Error::InvalidPdfObject("image height must be a positive integer > 0"))?;
        Ok(Self(w))
    }
}
