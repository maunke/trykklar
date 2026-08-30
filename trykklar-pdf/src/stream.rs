//! Stream Objects

use lopdf::{Document, Object, ObjectId};

use crate::codec::TryFromObject;
use crate::dict::DictKey;
use crate::{Error, Result};

/// Filter
///
/// ISO 32000-1:2008 Table 5 – Entries common to all stream dictionaries
///
/// > (Optional) The name of a filter that shall be applied in processing the stream data found
/// > between the keywords stream and endstream, or an array of zero, one or several names. Multiple
/// > filters shall be specified in the order in which they are to be applied.
#[derive(Debug, Clone)]
pub struct StreamFilter(pub(crate) Vec<Result<FilterName>>);

impl StreamFilter {
    /// Get the ordered slice of [`FilterName`].
    pub fn get(&self) -> &[Result<FilterName>] {
        &self.0
    }

    /// Get the last filter.
    pub fn get_last(&self) -> Option<Result<FilterName>> {
        self.0.last().cloned()
    }
}

impl DictKey for StreamFilter {
    const KEY: &'static [u8] = b"Filter";
}

/// Standard Filter
///
/// ISO 32000-1:2008 Table 6 – Standard filters
#[derive(Debug, Clone, Copy)]
pub enum FilterName {
    /// > Decodes data encoded in an ASCII hexadecimal representation, reproducing the original
    /// > binary data.
    ASCIIHexDecode,
    /// > Decodes data encoded in an ASCII base-85 representation, reproducing the original binary
    /// > data.
    ASCII85Decode,
    /// > Decompresses data encoded using the LZW (Lempel-Ziv-Welch) adaptive compression method,
    /// > reproducing the original text or binary data.
    LZWDecode,
    /// > (PDF 1.2) Decompresses data encoded using the zlib/deflate compression method, reproducing
    /// > the original text or binary data.
    FlateDecode,
    /// > Decompresses data encoded using a byte-oriented run-length encoding algorithm, reproducing
    /// > the original text or binary data (typically monochrome image data, or any data that
    /// > contains frequent long runs of a single byte value).
    RunLengthDecode,
    /// > Decompresses data encoded using the CCITT facsimile standard, reproducing the original
    /// > data (typically monochrome image data at 1 bit per pixel).
    CCITTFaxDecode,
    /// > (PDF 1.4) Decompresses data encoded using the JBIG2 standard, reproducing the original
    /// > monochrome (1 bit per pixel) image data (or an approximation of that data).
    JBIG2Decode,
    /// > Decompresses data encoded using a DCT (discrete cosine transform) technique based on the
    /// > JPEG standard, reproducing image sample data that approximates the original data.
    DCTDecode,
    /// > (PDF 1.5) Decompresses data encoded using the wavelet-based JPEG2000 standard, reproducing
    /// > the original image data.
    JPXDecode,
    /// > (PDF 1.5) Decrypts data encrypted by a security handler, reproducing the data as it was
    /// > before encryption.
    Crypt,
}

impl TryFrom<&[u8]> for FilterName {
    type Error = Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let variant = match value {
            b"ASCIIHexDecode" => Self::ASCIIHexDecode,
            b"ASCII85Decode" => Self::ASCII85Decode,
            b"LZWDecode" => Self::LZWDecode,
            b"FlateDecode" => Self::FlateDecode,
            b"RunLengthDecode" => Self::RunLengthDecode,
            b"CCITTFaxDecode" => Self::CCITTFaxDecode,
            b"JBIG2Decode" => Self::JBIG2Decode,
            b"DCTDecode" => Self::DCTDecode,
            b"JPXDecode" => Self::JPXDecode,
            b"Crypt" => Self::Crypt,
            _ => return Err(Error::InvalidFilter),
        };
        Ok(variant)
    }
}

impl TryFromObject<'_> for StreamFilter {
    fn try_from_object(doc: &'_ Document, _id: Option<ObjectId>, obj: &'_ Object) -> Result<Self> {
        let names_vec: Vec<&[u8]> = match obj {
            Object::Name(name) => vec![name],
            Object::Array(arr) => {
                let mut elements = Vec::new();
                for el in arr.iter() {
                    let el_name = doc.dereference(el)?.1.as_name()?;
                    elements.push(el_name);
                }
                elements
            }
            _ => return Err(Error::InvalidPdfObject("/Filter must be a name or array")),
        };
        let names = names_vec
            .into_iter()
            .map(FilterName::try_from)
            .collect::<Vec<_>>();
        Ok(Self(names))
    }
}
