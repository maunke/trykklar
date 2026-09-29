//! PDF Datetime
use crate::codec::TryFromObject;
use crate::{Error, Result};
use lopdf::decode_text_string;
use std::num::NonZero;
use time::format_description::{BorrowedFormatItem, well_known};
use time::macros::format_description;
use time::parsing::Parsed;
use time::{Month, OffsetDateTime};

/// Date
///
/// ISO 32000-1:2008 7.9.4 Dates
///
/// > Date values used in a PDF shall conform to a standard date format, which closely follows that
/// > of the international standard ASN.1 (Abstract Syntax Notation One), defined in ISO/IEC 8824. A
/// > date shall be a text string of the form
///
/// > (D:YYYYMMDDHHmmSSOHH'mm)
#[derive(Debug, Clone)]
pub struct PdfDate(OffsetDateTime);

impl PdfDate {
    /// Returns the offset aware datetime.
    pub fn get(&self) -> &OffsetDateTime {
        &self.0
    }
}

impl PdfDate {
    /// Returns the rfc 3339 formatted string of the date.
    pub fn to_rfc3339(&self) -> Result<String> {
        match self.0.format(&well_known::Rfc3339) {
            Ok(d) => Ok(d),
            _ => Err(Error::DateFormat),
        }
    }

    /// Returns the rfc 2822 formatted string of the date.
    pub fn to_rfc2822(&self) -> Result<String> {
        match self.0.format(&well_known::Rfc2822) {
            Ok(d) => Ok(d),
            _ => Err(Error::DateFormat),
        }
    }

    /// Returns the ISO 8601 formatted string of the date.
    pub fn to_iso8601(&self) -> Result<String> {
        match self.0.format(&well_known::Iso8601::DEFAULT) {
            Ok(d) => Ok(d),
            _ => Err(Error::DateFormat),
        }
    }
}

const PDF_DATE: &[BorrowedFormatItem<'_>] = format_description!(
    version = 2,
    "D:[year][optional [[month][optional [[day][optional [[hour][optional [[minute][optional \
     [[second]]]]]]]]]]][optional [[first [Z[optional [00'00']][optional [00']]][[offset_hour \
     sign:mandatory][optional [']][optional [[offset_minute][optional [']]]]]]]]"
);

impl TryFrom<String> for PdfDate {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        let s = value.trim();
        let mut p = Parsed::new();
        match p.parse_items(s.as_bytes(), PDF_DATE) {
            Ok([]) => {}
            _ => return Err(Error::DateParsing),
        };
        // > The default values for MM and DD shall be both 01
        if p.month().is_none() {
            p.set_month(Month::January);
        }
        let one = NonZero::<u8>::try_from(1)?;
        if p.day().is_none() {
            p.set_day(one);
        }
        // > all other numerical fields shall default to zero values.
        if p.hour_24().is_none() {
            p.set_hour_24(0);
        }
        if p.minute().is_none() {
            p.set_minute(0);
        }
        if p.second().is_none() {
            p.set_second(0);
        }

        // > HH followed by APOSTROPHE (U+0027) (') shall be the absolute value of the offset from
        // > UT in hours (00–23)
        if p.offset_hour().is_some_and(|h| h.abs() > 23) {
            return Err(Error::DateParsing);
        }
        // > and the LATIN CAPITAL LETTER Z signifies that local time is equal to UT. If no UT
        // > information is specified, the relationship of the specified time to UT shall be
        // > considered to be GMT
        if s.contains("Z") || p.offset_hour().is_none() {
            p.set_offset_hour(0);
            if p.offset_minute_signed().is_none() {
                p.set_offset_minute_signed(0);
            }
        }
        let Ok(date) = OffsetDateTime::try_from(p) else {
            return Err(Error::DateParsing);
        };
        Ok(Self(date))
    }
}

impl TryFromObject<'_> for PdfDate {
    fn try_from_object(
        _doc: &'_ lopdf::Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'_ lopdf::Object,
    ) -> Result<Self> {
        let value = decode_text_string(obj)?;
        PdfDate::try_from(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn datetime() -> Result<()> {
        let checks = [
            ("D:199812231952-08'00", datetime!(1998-12-23 19:52 -8)),
            ("D:2026", datetime!(2026-01-01 00:00 +0)),
            ("D:199812231952Z", datetime!(1998-12-23 19:52 +0)),
            ("D:199812231952", datetime!(1998-12-23 19:52 +0)),
        ];
        checks.iter().for_each(|(d, t)| {
            let date = PdfDate::try_from(d.to_string()).unwrap();
            assert_eq!(date.get(), t);
        });
        let err_checks = [
            "D:199812231952-08'00 With REST",
            "Not starting with D D:199812231952-08'00",
            "D:20261",
        ];
        err_checks.iter().for_each(|c| {
            assert!(PdfDate::try_from(c.to_string()).is_err());
        });
        Ok(())
    }
}
