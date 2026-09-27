//! Document Information Dictionary

use crate::codec::TryFromObject;
use crate::dict::{DictKey, read_optional_field};
use crate::error::OptionalField;
use crate::{Error, Result, object_id};
use chrono::{DateTime, FixedOffset, Utc};
use lopdf::{Dictionary, Document, decode_text_string};

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
    pub(crate) doc: &'a Document,
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
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`Author`].
    pub fn author(&self) -> OptionalField<Author> {
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`Subject`].
    pub fn subject(&self) -> OptionalField<Subject> {
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`Keywords`].
    pub fn keywords(&self) -> OptionalField<Keywords> {
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`Creator`].
    pub fn creator(&self) -> OptionalField<Creator> {
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`Producer`].
    pub fn producer(&self) -> OptionalField<Producer> {
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`CreationDate`].
    pub fn creation_date(&self) -> OptionalField<CreationDate> {
        read_optional_field(self.doc, self.dict)
    }

    /// Returns the [`ModDate`].
    pub fn mod_date(&self) -> OptionalField<ModDate> {
        read_optional_field(self.doc, self.dict)
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
}

impl DictKey for Title {
    const KEY: &'static [u8] = b"Title";
}

impl TryFromObject<'_> for Title {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let title = decode_text_string(obj)?;
        Ok(Self(title))
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
}

impl DictKey for Author {
    const KEY: &'static [u8] = b"Author";
}

impl TryFromObject<'_> for Author {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let author_name = decode_text_string(obj)?;
        Ok(Self(author_name))
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
}

impl DictKey for Subject {
    const KEY: &'static [u8] = b"Subject";
}

impl TryFromObject<'_> for Subject {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let subject = decode_text_string(obj)?;
        Ok(Self(subject))
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
}

impl DictKey for Keywords {
    const KEY: &'static [u8] = b"Keywords";
}

impl TryFromObject<'_> for Keywords {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let keywords = decode_text_string(obj)?;
        Ok(Self(keywords))
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
}

impl DictKey for Creator {
    const KEY: &'static [u8] = b"Creator";
}

impl TryFromObject<'_> for Creator {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let creator = decode_text_string(obj)?;
        Ok(Self(creator))
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
}

impl DictKey for Producer {
    const KEY: &'static [u8] = b"Producer";
}

impl TryFromObject<'_> for Producer {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let producer = decode_text_string(obj)?;
        Ok(Self(producer))
    }
}

/// `/CreationDate`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > (Optional) The date and time the document was created, in human-readable form (see 7.9.4,
/// > “Dates”).
pub struct CreationDate(DateTime<Utc>);

impl CreationDate {
    /// Returns the creation date.
    pub fn get(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl DictKey for CreationDate {
    const KEY: &'static [u8] = b"CreationDate";
}

impl TryFromObject<'_> for CreationDate {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let Some(creation_date_raw) = obj.as_datetime() else {
            return Err(Error::InvalidPdfObject("datetime is not provided"));
        };
        let creation_date_offset: DateTime<FixedOffset> = match creation_date_raw.try_into() {
            Ok(t) => t,
            Err(_) => return Err(Error::InvalidPdfObject("datetime not valid")),
        };

        let creation_date = creation_date_offset.into();
        Ok(Self(creation_date))
    }
}

/// `/ModDate`
///
/// ISO 32000-1:2008 Table 317 – Entries in the document information dictionary
///
/// > Required if PieceInfo is present in the document catalogue; otherwise optional; PDF 1.1) The
/// > date and time the document was most recently modified, in human-readable form (see 7.9.4,
/// > “Dates”).
pub struct ModDate(DateTime<Utc>);

impl ModDate {
    /// Returns the modified date.
    pub fn get(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl DictKey for ModDate {
    const KEY: &'static [u8] = b"ModDate";
}

impl TryFromObject<'_> for ModDate {
    fn try_from_object(
        _doc: &'_ Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let Some(mod_date_raw) = obj.as_datetime() else {
            return Err(Error::InvalidPdfObject("datetime is not provided"));
        };
        let mod_date_offset: DateTime<FixedOffset> = match mod_date_raw.try_into() {
            Ok(t) => t,
            Err(_) => return Err(Error::InvalidPdfObject("datetime not valid")),
        };

        let mod_date = mod_date_offset.into();
        Ok(Self(mod_date))
    }
}

#[cfg(test)]
mod tests {
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
            creation_date.get().to_rfc3339(),
            "2022-08-17T08:13:02+00:00"
        );
        Ok(())
    }
}
