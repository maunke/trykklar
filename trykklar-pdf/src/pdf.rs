//! PDF Document
use crate::dict::read_optional_field;
use crate::ocg::OCProperties;
use crate::page::{PdfPage, PdfPageId, PdfPageMut};
use crate::{Error, Result, object_id};
use lopdf::{Dictionary, Document, Object};

/// PDF
///
/// It follows the ISO 32000-1:2008 specification.
pub struct Pdf {
    doc: Document,
}

impl Pdf {
    /// Initializes the PDF from document.
    pub fn from_doc(doc: Document) -> Self {
        Self { doc }
    }

    /// Returns the underlying document.
    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// Returns the underlying mutable document.
    pub(crate) fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    /// Returns the pdf catalog.
    pub fn catalog(&self) -> Result<Catalog<'_>> {
        match self.doc.trailer.get(b"Root") {
            Ok(obj) => match self.doc.dereference(obj) {
                Ok((Some(id), Object::Dictionary(dict))) => Ok(Catalog {
                    doc: &self.doc,
                    id: CatalogId(id),
                    dict,
                }),
                _ => Err(Error::InvalidPdfObject("catalog needs to be a dictionary")),
            },
            Err(_) => Err(Error::CatalogNotFound),
        }
    }

    /// Load [Pdf] from `path`.
    pub fn load(path: &str) -> Result<Self> {
        let doc = Document::load(path)?;
        Ok(Self { doc })
    }

    /// Load [Pdf] from bytes slice as `data`.
    pub fn load_from_data(data: &[u8]) -> Result<Self> {
        let doc = Document::load_mem(data)?;
        Ok(Self { doc })
    }

    /// Returns the PDF version.
    pub fn version(&self) -> &str {
        &self.doc.version
    }

    /// Returns a list of PdfPage.
    pub fn pages(&self) -> Vec<Result<PdfPage<'_>>> {
        self.doc
            .get_pages()
            .into_values()
            .map(|id| PdfPage::new(&self.doc, PdfPageId::new(id)))
            .collect::<Vec<_>>()
    }

    /// Returns the page.
    pub fn page(&self, number: u32) -> Result<PdfPage<'_>> {
        let lopdf_number = number.checked_add(1).ok_or(Error::PageNotFound)?;
        match self.doc.get_pages().get(&lopdf_number) {
            Some(&id) => Ok(PdfPage::new(&self.doc, PdfPageId::new(id))?),
            None => Err(Error::PageNotFound),
        }
    }

    /// Returns the mutable page.
    pub fn page_mut(&mut self, number: u32) -> Result<PdfPageMut<'_>> {
        let lopdf_number = number.checked_add(1).ok_or(Error::PageNotFound)?;
        match self.doc.get_pages().get(&lopdf_number) {
            Some(&id) => Ok(PdfPageMut::new(&mut self.doc, PdfPageId::new(id))),
            None => Err(Error::PageNotFound),
        }
    }
}

object_id!(CatalogId);

/// PDF catalog
pub struct Catalog<'a> {
    doc: &'a Document,
    id: CatalogId,
    dict: &'a Dictionary,
}

impl<'a> Catalog<'a> {
    /// Returns the catalog id object.
    pub fn id(&self) -> CatalogId {
        self.id
    }

    /// Returns the oc properties.
    pub fn oc_properties(&self) -> Option<Result<OCProperties<'a>>> {
        read_optional_field::<OCProperties>(self.doc, self.dict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use lopdf::{Stream, dictionary};

    fn get_pdf() -> Pdf {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content = Content { operations: vec![] };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        let page_id = doc.add_object(
            dictionary! {"Type" => "Page", "Parent" => pages_id, "Contents" => content_id},
        );
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
        Pdf { doc }
    }

    #[test]
    fn load_from_path() -> Result<()> {
        assert!(Pdf::load("tests/assets/smallest-possible-pdf-1.0.pdf").is_ok());
        Ok(())
    }

    #[test]
    fn load_from_bytes() -> Result<()> {
        let data = std::fs::read("tests/assets/smallest-possible-pdf-1.0.pdf")?;
        assert!(Pdf::load_from_data(&data).is_ok());
        Ok(())
    }

    #[test]
    fn pdf_version() -> Result<()> {
        let pdf = Pdf::load("tests/assets/smallest-possible-pdf-1.0.pdf")?;
        assert_eq!("1.0", pdf.version());
        Ok(())
    }

    #[test]
    fn pdf_page_exists() -> Result<()> {
        let pdf = get_pdf();
        assert!(pdf.page(0).is_ok());
        // Error page numbers
        [1, 2, u32::MAX].iter().for_each(|&number| {
            assert!(matches!(pdf.page(number), Err(Error::PageNotFound)));
        });
        Ok(())
    }
}
