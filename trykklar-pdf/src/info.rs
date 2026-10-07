//! Document Information Dictionary

use crate::codec::{TryFromObject, TryIntoObject};
use crate::datetime::PdfDate;
use crate::dict::{self, DictKey, read_optional_field};
use crate::error::OptionalField;
use crate::{Error, Pdf, Result, object_id};
use lopdf::{Dictionary, Object, decode_text_string, text_string};

object_id!(InfoId);

/// Document Information Dictionary
///
/// ISO 32000-1:2008 14.3.3 Document Information Dictionary
///
/// > The optional Info entry in the trailer of a PDF file (see 7.5.5, “File Trailer”) shall hold a
/// > document information dictionary containing metadata for the document.
///
/// Table 317 – Entries in the document information dictionary
pub struct Info<'a> {
    pub(crate) pdf: &'a Pdf,
    pub(crate) id: InfoId,
    pub(crate) dict: &'a Dictionary,
}

impl<'a> Info<'a> {
    /// Returns the ID.
    pub fn id(&self) -> InfoId {
        self.id
    }

    /// Returns the [`Title`].
    pub fn title(&self) -> OptionalField<Title> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`Author`].
    pub fn author(&self) -> OptionalField<Author> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`Subject`].
    pub fn subject(&self) -> OptionalField<Subject> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`Keywords`].
    pub fn keywords(&self) -> OptionalField<Keywords> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`Creator`].
    pub fn creator(&self) -> OptionalField<Creator> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`Producer`].
    pub fn producer(&self) -> OptionalField<Producer> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`CreationDate`].
    pub fn creation_date(&self) -> OptionalField<CreationDate> {
        read_optional_field(self.pdf, self.dict)
    }

    /// Returns the [`ModDate`].
    pub fn mod_date(&self) -> OptionalField<ModDate> {
        read_optional_field(self.pdf, self.dict)
    }
}

impl DictKey for Info<'_> {
    const KEY: &'static [u8] = b"Info";
}

impl<'a> TryFromObject<'a> for Info<'a> {
    fn try_from_object(pdf: &'a Pdf, id: Option<lopdf::ObjectId>, obj: &'a Object) -> Result<Self> {
        let dict = obj.as_dict()?;
        let Some(obj_id) = id else {
            return Err(Error::InvalidPdfObject(
                "Info must be an indirect reference",
            ));
        };
        let id = InfoId(obj_id);
        Ok(Self { pdf, id, dict })
    }
}

/// `/Title`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional; PDF 1.1) The document’s title.
pub struct Title(String);

impl Title {
    /// Returns the title.
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Creates a new title.
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl DictKey for Title {
    const KEY: &'static [u8] = b"Title";
}

impl TryFromObject<'_> for Title {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let title = decode_text_string(obj)?;
        Ok(Self(title))
    }
}

impl TryIntoObject for Title {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/Author`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional) The name of the person who created the document.
pub struct Author(String);

impl Author {
    /// Returns the author name.
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Creates a new author.
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl DictKey for Author {
    const KEY: &'static [u8] = b"Author";
}

impl TryFromObject<'_> for Author {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let author_name = decode_text_string(obj)?;
        Ok(Self(author_name))
    }
}

impl TryIntoObject for Author {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/Subject`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional; PDF 1.1) The subject of the document.
pub struct Subject(String);

impl Subject {
    /// Returns the subject.
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Creates a new subject.
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl DictKey for Subject {
    const KEY: &'static [u8] = b"Subject";
}

impl TryFromObject<'_> for Subject {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let subject = decode_text_string(obj)?;
        Ok(Self(subject))
    }
}

impl TryIntoObject for Subject {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/Keywords`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional; PDF 1.1) Keywords associated with the document.
pub struct Keywords(String);

impl Keywords {
    /// Returns the keywords.
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Creates a new keywords objects.
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl DictKey for Keywords {
    const KEY: &'static [u8] = b"Keywords";
}

impl TryFromObject<'_> for Keywords {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let keywords = decode_text_string(obj)?;
        Ok(Self(keywords))
    }
}

impl TryIntoObject for Keywords {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/Creator`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional) If the document was converted to PDF from another format, the name of the
/// > conforming product that created the original document from which it was converted.
pub struct Creator(String);

impl Creator {
    /// Returns the creator.
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Creates a new creator.
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl DictKey for Creator {
    const KEY: &'static [u8] = b"Creator";
}

impl TryFromObject<'_> for Creator {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let creator = decode_text_string(obj)?;
        Ok(Self(creator))
    }
}

impl TryIntoObject for Creator {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/Producer`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional) If the document was converted to PDF from another format, the name of the
/// > conforming product that converted it to PDF.
pub struct Producer(String);

impl Producer {
    /// Returns the producer.
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Creates a new producer.
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl DictKey for Producer {
    const KEY: &'static [u8] = b"Producer";
}

impl TryFromObject<'_> for Producer {
    fn try_from_object(
        _pdf: &'_ Pdf,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let producer = decode_text_string(obj)?;
        Ok(Self(producer))
    }
}

impl TryIntoObject for Producer {
    fn try_into_object(self, _pdf: &mut Pdf) -> Result<Object> {
        Ok(text_string(self.get()))
    }
}

/// `/CreationDate`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional) The date and time the document was created, in human-readable form (see 7.9.4,
/// > “Dates”).
pub struct CreationDate(PdfDate);

impl CreationDate {
    /// Returns the creation date.
    pub fn get(&self) -> &PdfDate {
        &self.0
    }

    /// Creates a new creation date.
    pub fn new(value: PdfDate) -> Self {
        Self(value)
    }
}

impl DictKey for CreationDate {
    const KEY: &'static [u8] = b"CreationDate";
}

impl TryFromObject<'_> for CreationDate {
    fn try_from_object(
        pdf: &'_ Pdf,
        id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let creation_date = PdfDate::try_from_object(pdf, id, obj)?;
        Ok(Self(creation_date))
    }
}

impl TryIntoObject for CreationDate {
    fn try_into_object(self, pdf: &mut Pdf) -> Result<Object> {
        self.0.try_into_object(pdf)
    }
}

/// `/ModDate`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > Required if PieceInfo is present in the document catalogue; otherwise optional; PDF 1.1) The
/// > date and time the document was most recently modified, in human-readable form (see 7.9.4,
/// > “Dates”).
pub struct ModDate(PdfDate);

impl ModDate {
    /// Returns the modified date.
    pub fn get(&self) -> &PdfDate {
        &self.0
    }

    /// reates a new modified date.
    pub fn new(value: PdfDate) -> Self {
        Self(value)
    }
}

impl DictKey for ModDate {
    const KEY: &'static [u8] = b"ModDate";
}

impl TryFromObject<'_> for ModDate {
    fn try_from_object(
        pdf: &'_ Pdf,
        id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let mod_date = PdfDate::try_from_object(pdf, id, obj)?;
        Ok(Self(mod_date))
    }
}

impl TryIntoObject for ModDate {
    fn try_into_object(self, pdf: &mut Pdf) -> Result<Object> {
        self.0.try_into_object(pdf)
    }
}

/// Mutation object for an [`Info`] by providing the [`crate::Pdf`] and [`InfoId`].
pub struct InfoMut<'a> {
    pdf: &'a mut Pdf,
    id: InfoId,
}

impl<'a> InfoMut<'a> {
    /// Creates the mutation object for a given [`InfoId`].
    pub fn try_new(pdf: &'a mut Pdf, id: InfoId) -> Result<Self> {
        dict::exists(id, pdf)?;
        Ok(Self { pdf, id })
    }

    /// Creates a new information dictionary.
    pub fn create(pdf: &'a mut Pdf) -> Self {
        let obj = Dictionary::new();
        let obj_id = pdf.doc_mut().add_object(obj);
        pdf.doc_mut()
            .trailer
            .set(Info::KEY, Object::Reference(obj_id));
        let id = InfoId(obj_id);
        Self { pdf, id }
    }

    /// Gets or creates a trailer information dictionary.
    pub fn get_or_create(pdf: &'a mut Pdf) -> Result<Self> {
        match pdf.info() {
            Some(info) => {
                let id = info?.id();
                Self::try_new(pdf, id)
            }
            None => Ok(Self::create(pdf)),
        }
    }

    /// Sets the title.
    pub fn set_title(&mut self, title: Title) -> Result<()> {
        dict::write(title, self.pdf, |pdf: &mut Pdf| dict::get_mut(self.id, pdf))
    }

    /// Sets the author.
    pub fn set_author(&mut self, author: Author) -> Result<()> {
        dict::write(author, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the subject.
    pub fn set_subject(&mut self, subject: Subject) -> Result<()> {
        dict::write(subject, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the keywords.
    pub fn set_keywords(&mut self, keywords: Keywords) -> Result<()> {
        dict::write(keywords, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the creator.
    pub fn set_creator(&mut self, creator: Creator) -> Result<()> {
        dict::write(creator, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the producer.
    pub fn set_producer(&mut self, producer: Producer) -> Result<()> {
        dict::write(producer, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the creation date.
    pub fn set_creation_date(&mut self, creation_date: CreationDate) -> Result<()> {
        dict::write(creation_date, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }

    /// Sets the mod date.
    pub fn set_mod_date(&mut self, mod_date: ModDate) -> Result<()> {
        dict::write(mod_date, self.pdf, |pdf: &mut Pdf| {
            dict::get_mut(self.id, pdf)
        })
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;
    use crate::Pdf;

    #[test]
    fn info() -> Result<()> {
        let pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let info = pdf.info().expect("info must exist")?;

        let title = info.title().expect("title must exist")?;
        assert_eq!(title.get(), "Hierarchical Layers");

        let producer = info.producer().expect("producer must exist")?;
        assert_eq!(
            producer.get(),
            "PDFlib Personalization Server 10.0.0p5 (Java/macOS (x64))"
        );

        let creation_date = info.creation_date().expect("creation_date must exist")?;
        assert_eq!(
            creation_date.get().to_rfc3339()?,
            "2022-08-17T10:13:02+02:00"
        );
        Ok(())
    }

    #[test]
    fn info_mut() -> Result<()> {
        let mut pdf = Pdf::load("tests/assets/hierarchical_layers.pdf")?;
        let info = pdf.info().expect("info should exist here")?;
        let info_id = info.id();
        let mut info_mut = InfoMut::try_new(&mut pdf, info_id)?;

        let title_name = "Awesome Artwork";
        let title = Title::new(title_name);
        info_mut.set_title(title)?;

        let author_name = "Markus Unkel";
        let author = Author::new(author_name);
        info_mut.set_author(author)?;

        let subject_name = "This is an information about the packaging product.";
        let subject = Subject::new(subject_name);
        info_mut.set_subject(subject)?;

        let keywords_name = "Pouch;4C+White";
        let keywords = Keywords::new(keywords_name);
        info_mut.set_keywords(keywords)?;

        let creator_name = "trykklar";
        let creator = Creator::new(creator_name);
        info_mut.set_creator(creator)?;

        let producer_name = "trykklar";
        let producer = Producer::new(producer_name);
        info_mut.set_producer(producer)?;

        let date = datetime!(2026-10-03 13:13:42 +0);
        let pdf_date = PdfDate::try_new(date)?;

        let creation_date = CreationDate::new(pdf_date.clone());
        info_mut.set_creation_date(creation_date)?;

        let mod_date = ModDate::new(pdf_date.clone());
        info_mut.set_mod_date(mod_date)?;

        // check the updated values
        let info = pdf.info().expect("info should exist here")?;

        let title = info.title().expect("title should exist")?;
        assert_eq!(title_name, title.get());

        let author = info.author().expect("author should exist")?;
        assert_eq!(author_name, author.get());

        let subject = info.subject().expect("subject should exist")?;
        assert_eq!(subject_name, subject.get());

        let keywords = info.keywords().expect("keywords should exist")?;
        assert_eq!(keywords_name, keywords.get());

        let creator = info.creator().expect("creator should exist")?;
        assert_eq!(creator_name, creator.get());

        let producer = info.producer().expect("producer should exist")?;
        assert_eq!(producer_name, producer.get());

        let creation_date = info.creation_date().expect("creation_date should exist")?;
        assert_eq!(pdf_date, *creation_date.get());

        let mod_date = info.mod_date().expect("mod_date should exist")?;
        assert_eq!(pdf_date, *mod_date.get());

        Ok(())
    }
}
