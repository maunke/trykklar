//! PDF Page
use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::codec::{IntoObject, TryFromObject};
use crate::dict::{self, DictKey, read_field, read_optional_field};
use crate::error::FieldExt;
use crate::geometry::Rect;
use crate::unit::{UserSpace, UserUnit};
use crate::{Error, PhysicalUnit, Result, object_id};

object_id!(PdfPageId);

/// PDF Page
#[derive(Debug, Clone)]
pub struct PdfPage<'a> {
    doc: &'a Document,
    id: PdfPageId,
    dict: &'a Dictionary,
}

impl<'a> PdfPage<'a> {
    pub(crate) fn new(doc: &'a Document, id: PdfPageId) -> Result<Self> {
        let dict = doc.get_dictionary(id.get())?;
        Ok(Self { doc, id, dict })
    }

    /// Returns the page object id.
    pub fn id(&self) -> PdfPageId {
        self.id
    }

    pub(crate) fn doc(&self) -> &'a Document {
        self.doc
    }

    /// Returns the user unit.
    pub fn user_unit(&self) -> Result<UserUnit> {
        match read_optional_field::<UserUnit>(self.doc, self.dict) {
            Some(value) => value,
            _ => Ok(UserUnit::default()),
        }
    }

    pub(crate) fn media_box_user(&self) -> Result<MediaBox<UserSpace>> {
        read_field::<MediaBox<UserSpace>>(self.doc, self.dict).as_result()
    }

    /// Returns the media box.
    pub fn media_box<U: PhysicalUnit>(&self) -> Result<MediaBox<U>> {
        let media_box = read_field::<MediaBox<UserSpace>>(self.doc, self.dict)?;
        let rect = media_box.get();
        let uu = self.user_unit()?;
        Ok(MediaBox(rect.to_physical(uu)))
    }

    /// Returns the crop box.
    pub fn crop_box<U: PhysicalUnit>(&self) -> Result<CropBox<U>> {
        match read_optional_field::<CropBox<UserSpace>>(self.doc, self.dict) {
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
        match read_optional_field::<BleedBox<UserSpace>>(self.doc, self.dict) {
            Some(value) => {
                let bleed_box = value?;
                let rect = bleed_box.get();
                let uu = self.user_unit()?;
                Ok(BleedBox(rect.to_physical(uu)))
            }
            None => {
                // Default of CropBox is MediaBox
                let media_box = self.crop_box()?;
                let rect = media_box.get();
                Ok(BleedBox(rect))
            }
        }
    }

    /// Returns the trim box.
    pub fn trim_box<U: PhysicalUnit>(&self) -> Result<TrimBox<U>> {
        match read_optional_field::<TrimBox<UserSpace>>(self.doc, self.dict) {
            Some(value) => {
                let trim_box = value?;
                let rect = trim_box.get();
                let uu = self.user_unit()?;
                Ok(TrimBox(rect.to_physical(uu)))
            }
            None => {
                // Default of CropBox is MediaBox
                let media_box = self.crop_box()?;
                let rect = media_box.get();
                Ok(TrimBox(rect))
            }
        }
    }
}

/// Mutable page object.
pub struct PdfPageMut<'a> {
    doc: &'a mut Document,
    id: PdfPageId,
}

impl<'a> PdfPageMut<'a> {
    pub(crate) fn new(doc: &'a mut Document, id: PdfPageId) -> Self {
        Self { doc, id }
    }

    /// Reborrow as a shared view so read methods aren't duplicated.
    fn as_page(&self) -> Result<PdfPage<'_>> {
        PdfPage::new(self.doc, self.id)
    }

    /// Sets the media box.
    pub fn set_media_box<U: PhysicalUnit>(&mut self, value: MediaBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let media_box: MediaBox<UserSpace> = rect.into();
        let page = self.doc.get_dictionary_mut(self.id.get())?;
        dict::write(media_box, page);
        Ok(())
    }

    /// Sets the crop box.
    pub fn set_crop_box<U: PhysicalUnit>(&mut self, value: CropBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let crop_box: CropBox<UserSpace> = rect.into();
        let page = self.doc.get_dictionary_mut(self.id.get())?;
        dict::write(crop_box, page);
        Ok(())
    }

    /// Sets the bleed box.
    pub fn set_bleed_box<U: PhysicalUnit>(&mut self, value: BleedBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let bleed_box: BleedBox<UserSpace> = rect.into();
        let page = self.doc.get_dictionary_mut(self.id.get())?;
        dict::write(bleed_box, page);
        Ok(())
    }

    /// Sets the trim box.
    pub fn set_trim_box<U: PhysicalUnit>(&mut self, value: TrimBox<U>) -> Result<()> {
        let uu = self.as_page()?.user_unit()?;
        let rect: Rect<UserSpace> = value.get().to_user(uu);
        let trim_box: TrimBox<UserSpace> = rect.into();
        let page = self.doc.get_dictionary_mut(self.id.get())?;
        dict::write(trim_box, page);
        Ok(())
    }
}

impl DictKey for UserUnit {
    const KEY: &'static [u8] = b"UserUnit";
}

impl TryFromObject<'_> for UserUnit {
    fn try_from_object(_doc: &Document, _id: Option<ObjectId>, obj: &Object) -> Result<Self> {
        Self::try_from(obj.as_float()? as f64)
    }
}

impl IntoObject for UserUnit {
    fn into_object(self) -> Object {
        Object::Real(self.get() as f32)
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
            fn try_from_object(
                _doc: &Document,
                _id: Option<ObjectId>,
                obj: &Object,
            ) -> Result<Self> {
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

        impl IntoObject for $name<UserSpace> {
            fn into_object(self) -> Object {
                Object::Array(
                    self.get()
                        .as_box_slice()
                        .iter()
                        .map(|&v| Object::Real(v as f32))
                        .collect(),
                )
            }
        }
    };
}

page_box!(MediaBox, b"MediaBox", inheritable: true);
page_box!(CropBox, b"CropBox", inheritable: true);
page_box!(BleedBox, b"BleedBox", inheritable: false);
page_box!(TrimBox, b"TrimBox", inheritable: false);

#[cfg(test)]
mod tests {

    use lopdf::content::Content;
    use lopdf::{Stream, dictionary};

    use crate::geometry::{Point, Size};
    use crate::pdf::Pdf;
    use crate::{Length, Mm, Pt};

    use super::*;

    // 2^-23, one ulp — 2× over the half-ulp floor
    const F32_ROUNDTRIP_REL: f64 = 1.19e-7;

    fn approx_eq(a: f64, b: f64) {
        assert!((a - b).abs() <= F32_ROUNDTRIP_REL * a.abs().max(1.0))
    }

    fn get_pdf(user_unit: Option<f64>) -> Pdf {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content = Content { operations: vec![] };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        let mut page =
            dictionary! {"Type" => "Page", "Parent" => pages_id, "Contents" => content_id};
        // Set the UserUnit entry on the page dictionary.
        if let Some(value) = user_unit {
            dict::write(UserUnit::try_from(value).expect("correct"), &mut page);
        }
        let page_id = doc.add_object(page);

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };

        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });

        doc.trailer.set("Root", catalog_id);
        Pdf::from_doc(doc)
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
        let pages_dict = doc.get_dictionary_mut(pages_dict_id)?;

        let size = Size::<Mm> {
            width: 155.0.try_into()?,
            height: 204.0.try_into()?,
        };
        let origin = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let media_box: MediaBox<UserSpace> = Rect { size, origin }.to_user(uu).into();

        dict::write(media_box, pages_dict);

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
}
