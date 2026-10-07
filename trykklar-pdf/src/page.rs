//! PDF Page
use lopdf::{Dictionary, Object, ObjectId};

use crate::codec::{TryFromObject, TryIntoObject};
use crate::dict::{self, DictKey, read_field, read_optional_field};
use crate::error::FieldExt;
use crate::geometry::Rect;
use crate::unit::{UserSpace, UserUnit};
use crate::{Error, ObjectAsF64, Pdf, PhysicalUnit, Result, object_id};

object_id!(PdfPageId);

/// PDF Page
#[derive(Debug, Clone)]
pub struct PdfPage<'a> {
    pdf: &'a Pdf,
    id: PdfPageId,
    dict: &'a Dictionary,
}

impl<'a> PdfPage<'a> {
    pub(crate) fn new(pdf: &'a Pdf, id: PdfPageId) -> Result<Self> {
        let dict = pdf.doc().get_dictionary(id.get())?;
        Ok(Self { pdf, id, dict })
    }

    pub(crate) fn pdf(&self) -> &Pdf {
        self.pdf
    }

    /// Returns the page object id.
    pub fn id(&self) -> PdfPageId {
        self.id
    }

    /// Returns the user unit.
    pub fn user_unit(&self) -> Result<UserUnit> {
        match read_optional_field::<UserUnit>(self.pdf, self.dict) {
            Some(value) => value,
            _ => Ok(UserUnit::default()),
        }
    }

    /// Returns the page rotation.
    pub fn rotation(&self) -> Result<PageRotate> {
        match read_optional_field(self.pdf, self.dict) {
            Some(value) => value,
            None => Ok(Default::default()),
        }
    }

    pub(crate) fn media_box_user(&self) -> Result<MediaBox<UserSpace>> {
        read_field::<MediaBox<UserSpace>>(self.pdf, self.dict).as_result()
    }

    /// Returns the media box.
    pub fn media_box<U: PhysicalUnit>(&self) -> Result<MediaBox<U>> {
        let media_box = read_field::<MediaBox<UserSpace>>(self.pdf, self.dict)?;
        let rect = media_box.get();
        let uu = self.user_unit()?;
        Ok(MediaBox(rect.to_physical(uu)))
    }

    /// Returns the crop box.
    pub fn crop_box<U: PhysicalUnit>(&self) -> Result<CropBox<U>> {
        match read_optional_field::<CropBox<UserSpace>>(self.pdf, self.dict) {
            Some(value) => {
                let crop_box = value?;
                let rect = crop_box.get();
                let uu = self.user_unit()?;
                Ok(CropBox(rect.to_physical(uu)))
            }
            None => {
                // Default of CropBox is MediaBox
                let media_box = self.media_box()?;
                let rect = media_box.get();
                Ok(CropBox(rect))
            }
        }
    }

    /// Returns the bleed box.
    pub fn bleed_box<U: PhysicalUnit>(&self) -> Result<BleedBox<U>> {
        match read_optional_field::<BleedBox<UserSpace>>(self.pdf, self.dict) {
            Some(value) => {
                let bleed_box = value?;
                let rect = bleed_box.get();
                let uu = self.user_unit()?;
                Ok(BleedBox(rect.to_physical(uu)))
            }
            None => {
                // Default of BleedBox is CropBox
                let crop_box = self.crop_box()?;
                let rect = crop_box.get();
                Ok(BleedBox(rect))
            }
        }
    }

    /// Returns the trim box.
    pub fn trim_box<U: PhysicalUnit>(&self) -> Result<TrimBox<U>> {
        match read_optional_field::<TrimBox<UserSpace>>(self.pdf, self.dict) {
            Some(value) => {
                let trim_box = value?;
                let rect = trim_box.get();
                let uu = self.user_unit()?;
                Ok(TrimBox(rect.to_physical(uu)))
            }
            None => {
                // Default of TrimBox is CropBox
                let crop_box = self.crop_box()?;
                let rect = crop_box.get();
                Ok(TrimBox(rect))
            }
        }
    }

    /// Returns the art box.
    pub fn art_box<U: PhysicalUnit>(&self) -> Result<ArtBox<U>> {
        match read_optional_field::<ArtBox<UserSpace>>(self.pdf, self.dict) {
            Some(value) => {
                let art_box = value?;
                let rect = art_box.get();
                let uu = self.user_unit()?;
                Ok(ArtBox(rect.to_physical(uu)))
            }
            None => {
                // Default of ArtBox is CropBox
                let crop_box = self.crop_box()?;
                let rect = crop_box.get();
                Ok(ArtBox(rect))
            }
        }
    }
}

/// Mutable page object.
pub struct PdfPageMut<'a> {
    pdf: &'a mut Pdf,
    id: PdfPageId,
}

impl<'a> PdfPageMut<'a> {
    pub(crate) fn new(pdf: &'a mut Pdf, id: PdfPageId) -> Self {
        Self { pdf, id }
    }

    /// Reborrow as a shared view so read methods aren't duplicated.
    fn as_page(&self) -> Result<PdfPage<'_>> {
        PdfPage::new(self.pdf, self.id)
    }

    /// Sets the media box.
    pub fn set_media_box<U: PhysicalUnit>(&mut self, value: MediaBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let media_box: MediaBox<UserSpace> = rect.into();
        dict::write(media_box, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the crop box.
    pub fn set_crop_box<U: PhysicalUnit>(&mut self, value: CropBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let crop_box: CropBox<UserSpace> = rect.into();
        dict::write(crop_box, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the bleed box.
    pub fn set_bleed_box<U: PhysicalUnit>(&mut self, value: BleedBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let bleed_box: BleedBox<UserSpace> = rect.into();
        dict::write(bleed_box, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the trim box.
    pub fn set_trim_box<U: PhysicalUnit>(&mut self, value: TrimBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let trim_box: TrimBox<UserSpace> = rect.into();
        dict::write(trim_box, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }
}

impl DictKey for UserUnit {
    const KEY: &'static [u8] = b"UserUnit";
}

impl TryFromObject<'_> for UserUnit {
    fn try_from_object(_pdf: &Pdf, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Self::try_from(obj.as_float()? as f64)
    }
}

impl TryIntoObject for UserUnit {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(Object::Real(self.get() as f32))
    }
}

impl Default for UserUnit {
    fn default() -> Self {
        Self(1.0)
    }
}

impl TryFrom<f64> for UserUnit {
    type Error = Error;
    fn try_from(value: f64) -> Result<Self> {
        if !(value.is_finite() && value > 0.) {
            return Err(Error::InvalidUserUnit { value });
        }
        Ok(Self(value))
    }
}

/// `/Rotate` Page Rotation
///
/// ISO 32000-1:2008 7.7.3.3 Page Objects Table 30 – Entries in a page object
///
/// > (Optional; inheritable) The number of degrees by which the page shall be rotated clockwise
/// > when displayed or printed. The value shall be a multiple of 90.
/// >
/// > Default value: 0.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum PageRotate {
    /// Rotate 0 degree clockwise
    #[default]
    R0,
    /// Rotate 90 degrees clockwise
    R90,
    /// Rotate 180 degrees clockwise
    R180,
    /// Rotate 280 degrees clockwise
    R270,
}

impl DictKey for PageRotate {
    const KEY: &'static [u8] = b"Rotate";
    const INHERITABLE: bool = true;
}

impl TryFromObject<'_> for PageRotate {
    fn try_from_object(_pdf: &'_ Pdf, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let degrees_f64 = obj.as_f64()?;
        Self::try_from(degrees_f64)
    }
}

impl TryFrom<f64> for PageRotate {
    type Error = Error;
    fn try_from(value: f64) -> std::result::Result<Self, Self::Error> {
        let degrees = if value.fract() == 0.0 {
            value as i64
        } else {
            return Err(Error::InvalidPdfObject(
                "rotate must be an integer or a float without a fractional part",
            ));
        };
        let value = match degrees.rem_euclid(360) {
            0 => Self::R0,
            90 => Self::R90,
            180 => Self::R180,
            270 => Self::R270,
            _ => {
                return Err(Error::InvalidPdfObject(
                    "page rotate must be a multiple of 90",
                ));
            }
        };
        Ok(value)
    }
}

macro_rules! page_box {
    ($name:ident, $key:literal, inheritable: $inherit:expr) => {
        /// Page Box
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name<U>(Rect<U>);

        impl<U> $name<U> {
            /// Returns the page box rectangle.
            #[inline]
            #[must_use]
            pub fn get(self) -> Rect<U> {
                self.0
            }
        }

        impl<U> From<Rect<U>> for $name<U> {
            fn from(value: Rect<U>) -> Self {
                Self(value)
            }
        }

        impl DictKey for $name<UserSpace> {
            const KEY: &'static [u8] = $key;
            const INHERITABLE: bool = $inherit;
        }

        impl TryFromObject<'_> for $name<UserSpace> {
            fn try_from_object(_pdf: &Pdf, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
                match obj {
                    Object::Array(array) => {
                        // Check for 4 entries
                        let [llx, lly, urx, ury] = &array[..] else {
                            return Err(Error::InvalidPdfObject(
                                "Page box should contain 4 array entries",
                            ));
                        };

                        let values = [
                            llx.as_float()? as f64,
                            lly.as_float()? as f64,
                            urx.as_float()? as f64,
                            ury.as_float()? as f64,
                        ];
                        let rect = Rect::<UserSpace>::try_from(values)?;
                        Ok(Self(rect))
                    }
                    _ => Err(Error::InvalidPdfObject("Page box value is not an array")),
                }
            }
        }

        impl TryIntoObject for $name<UserSpace> {
            fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
                Ok(Object::Array(
                    self.get()
                        .as_box_slice()
                        .iter()
                        .map(|&v| Object::Real(v as f32))
                        .collect(),
                ))
            }
        }
    };
}

page_box!(MediaBox, b"MediaBox", inheritable: true);
page_box!(CropBox, b"CropBox", inheritable: true);
page_box!(BleedBox, b"BleedBox", inheritable: false);
page_box!(TrimBox, b"TrimBox", inheritable: false);
page_box!(ArtBox, b"ArtBox", inheritable: false);

#[cfg(test)]
mod tests {

    use lopdf::content::Content;
    use lopdf::{Document, Stream, dictionary};

    use crate::geometry::{Point, Size};
    use crate::pdf::Pdf;
    use crate::{Length, Mm, Pt};

    use super::*;

    fn approx_eq(a: f64, b: f64) {
        // 2^-23 = one ulp 2 times over the half-ulp floor
        const F32_ROUNDTRIP_REL: f64 = 1.19e-7;
        assert!((a - b).abs() <= F32_ROUNDTRIP_REL * a.abs().max(1.0))
    }

    fn get_pdf(user_unit: Option<f64>) -> Pdf {
        let doc = Document::with_version("1.5");
        let mut pdf = Pdf::from_doc(doc);
        let pages_id = pdf.doc_mut().new_object_id();
        let content = Content { operations: vec![] };
        let content_id = pdf
            .doc_mut()
            .add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        let page = dictionary! {"Type" => "Page", "Parent" => pages_id, "Contents" => content_id};
        let page_id = pdf.doc_mut().add_object(page);
        // Set the UserUnit entry on the page dictionary.
        if let Some(value) = user_unit {
            dict::write(
                UserUnit::try_from(value).expect("correct"),
                &mut pdf,
                |pdf: &mut Pdf| dict::get_mut(page_id, pdf),
            )
            .expect("test pdf should be created");
        }

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };

        pdf.doc_mut()
            .objects
            .insert(pages_id, Object::Dictionary(pages));
        let catalog_id = pdf.doc_mut().add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });

        pdf.doc_mut().trailer.set("Root", catalog_id);
        pdf
    }

    #[test]
    fn user_unit() {
        // Correct
        [1e-9, 1.0, f64::MAX].iter().for_each(|&v| {
            let user_unit = UserUnit::try_from(v);
            assert!(user_unit.is_ok());
        });
        // Invalid User Unit
        [f64::NEG_INFINITY, f64::MIN, 0.0, f64::INFINITY]
            .iter()
            .for_each(|&v| {
                let user_unit = UserUnit::try_from(v);
                assert!(matches!(user_unit, Err(Error::InvalidUserUnit { .. })));
            });
    }

    #[test]
    fn page_user_unit() -> Result<()> {
        // Present and stored indirectly: must be dereferenced and read back.
        let pdf = get_pdf(Some(2.5));
        assert_eq!(pdf.page(0)?.user_unit()?.get(), 2.5);

        // Absent: falls back to the PDF spec default of 1.0: UserUnit::default().
        let pdf = get_pdf(None);
        assert_eq!(pdf.page(0)?.user_unit()?.get(), UserUnit::default().get());

        Ok(())
    }

    #[test]
    fn page_media_box() -> Result<()> {
        let pdf = get_pdf(None);
        let page = pdf.page(0)?;
        let media_box: MediaBox<Pt> = page.media_box()?;

        let size = Size {
            width: 595.0.try_into()?,
            height: 842.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let media_box_test: MediaBox<Pt> = Rect { size, origin }.into();
        assert_eq!(media_box, media_box_test);
        Ok(())
    }

    #[test]
    fn set_page_media_box() -> Result<()> {
        let mut pdf = get_pdf(None);
        let mut page = pdf.page_mut(0)?;

        let size = Size {
            width: 155.0.try_into()?,
            height: 204.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let media_box: MediaBox<Mm> = Rect { size, origin }.into();

        page.set_media_box(media_box)?;

        let page_media_box: MediaBox<Mm> = page.as_page()?.media_box()?;

        let mb_rect = media_box.get();
        let mb_page_rect = page_media_box.get();

        approx_eq(mb_rect.size.width.get(), mb_page_rect.size.width.get());
        approx_eq(mb_rect.size.height.get(), mb_page_rect.size.height.get());
        approx_eq(mb_rect.origin.x.get(), mb_page_rect.origin.x.get());
        approx_eq(mb_rect.origin.y.get(), mb_page_rect.origin.y.get());
        Ok(())
    }

    #[test]
    fn set_page_media_box_inheritable() -> Result<()> {
        let mut pdf = get_pdf(None);

        let page = pdf.page(0)?;
        let uu = page.user_unit()?;

        let doc = pdf.doc_mut();
        let catalog = doc.catalog_mut()?;
        let pages_dict_id = catalog.get(b"Pages")?.as_reference()?;

        let size = Size::<Mm> {
            width: 155.0.try_into()?,
            height: 204.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let media_box: MediaBox<UserSpace> = Rect { size, origin }.to_user(uu).into();

        dict::write(media_box, &mut pdf, |pdf: &mut Pdf| {
            dict::get_mut(pages_dict_id, pdf)
        })?;

        let page = pdf.page(0)?;
        let page_media_box: MediaBox<Mm> = page.media_box()?;

        let mb_rect: Rect<Mm> = media_box.get().to_physical(uu);
        let mb_page_rect = page_media_box.get();

        approx_eq(mb_rect.size.width.get(), mb_page_rect.size.width.get());
        approx_eq(mb_rect.size.height.get(), mb_page_rect.size.height.get());
        approx_eq(mb_rect.origin.x.get(), mb_page_rect.origin.x.get());
        approx_eq(mb_rect.origin.y.get(), mb_page_rect.origin.y.get());

        // Keep the pages MediaBox entry from before and check that Page dict is used when
        // writing to it
        let mut page = pdf.page_mut(0)?;

        let size = Size {
            width: 123.0.try_into()?,
            height: 456.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let media_box: MediaBox<Mm> = Rect { size, origin }.into();

        page.set_media_box(media_box)?;

        let page = pdf.page(0)?;
        let page_media_box: MediaBox<Mm> = page.media_box()?;

        let mb_rect = media_box.get();
        let mb_page_rect = page_media_box.get();

        approx_eq(mb_rect.size.width.get(), mb_page_rect.size.width.get());
        approx_eq(mb_rect.size.height.get(), mb_page_rect.size.height.get());
        approx_eq(mb_rect.origin.x.get(), mb_page_rect.origin.x.get());
        approx_eq(mb_rect.origin.y.get(), mb_page_rect.origin.y.get());

        Ok(())
    }

    #[test]
    fn crop_box_default() -> Result<()> {
        let pdf = get_pdf(None);
        let page = pdf.page(0)?;
        let media_box: MediaBox<Pt> = page.media_box()?;

        let size = Size {
            width: 595.0.try_into()?,
            height: 842.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let media_box_test: MediaBox<Pt> = Rect { size, origin }.into();
        assert_eq!(media_box, media_box_test);
        // Cropbox defaults to mediabox
        let crop_box: CropBox<Pt> = page.crop_box()?;
        let crop_box_test: CropBox<Pt> = Rect { size, origin }.into();
        assert_eq!(crop_box, crop_box_test);
        Ok(())
    }

    #[test]
    fn set_trim_box() -> Result<()> {
        let mut pdf = get_pdf(None);
        let mut page = pdf.page_mut(0)?;

        let size = Size {
            width: 155.0.try_into()?,
            height: 204.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let trim_box: TrimBox<Mm> = Rect { size, origin }.into();

        page.set_trim_box(trim_box)?;

        let page_trim_box: TrimBox<Mm> = page.as_page()?.trim_box()?;

        let tb_rect = trim_box.get();
        let tb_page_rect = page_trim_box.get();

        approx_eq(tb_rect.size.width.get(), tb_page_rect.size.width.get());
        approx_eq(tb_rect.size.height.get(), tb_page_rect.size.height.get());
        approx_eq(tb_rect.origin.x.get(), tb_page_rect.origin.x.get());
        approx_eq(tb_rect.origin.y.get(), tb_page_rect.origin.y.get());
        Ok(())
    }

    #[test]
    fn page_rotate() -> Result<()> {
        // correct
        [
            (0., PageRotate::R0),
            (90., PageRotate::R90),
            (-90., PageRotate::R270),
            (720., PageRotate::R0),
            (-1440., PageRotate::R0),
            (540., PageRotate::R180),
            (-630., PageRotate::R90),
            (630., PageRotate::R270),
        ]
        .into_iter()
        .for_each(|(deg, check)| assert_eq!(PageRotate::try_from(deg).unwrap(), check));

        // invalid
        [0.1, 1e-100, f64::NAN, f64::INFINITY]
            .into_iter()
            .for_each(|deg| assert!(PageRotate::try_from(deg).is_err()));

        // default value
        let pdf = get_pdf(None);
        assert_eq!(pdf.page(0)?.rotation()?, PageRotate::default());
        Ok(())
    }
}
