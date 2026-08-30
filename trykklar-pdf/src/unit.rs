//! User and Physical Space related types.
use crate::{Error, Result};
use std::marker::PhantomData;
use std::ops::{Add, Sub};

/// Marks [`Length`] related objects to represent the user space.
///
/// ISO 32000-1:2008 8.3.2.3 User Space
/// > To avoid the device-dependent effects of specifying objects in device space, PDF defines a
/// > device-independent coordinate system that always bears the same relationship to the current
/// > page, regardless of the output device on which printing or displaying occurs. This
/// > device-independent coordinate system is called user space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSpace;

/// Marks [`Length`] related objects represented in a physical unit.
///
/// The main purpose of this object is to prevent an interpretation of user space related lengths as
/// defined in physical units like pt, mm or inch. This enforces to map between [`UserSpace`] and
/// [`PhysicalUnit`] based lengths via the page dependent [`UserUnit`].
pub trait PhysicalUnit {
    /// The normalization factor between the present unit and pt.
    ///
    /// Example: From pt to pt, `TO_POINTS` must be `1`.
    const TO_POINTS: f64;
}

/// Marks [`Length`] in millimeter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mm;

impl PhysicalUnit for Mm {
    const TO_POINTS: f64 = 72.0 / 25.4;
}

/// Marks [`Length`] in points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pt;

impl PhysicalUnit for Pt {
    const TO_POINTS: f64 = 1.0;
}

/// Marks [`Length`] in inch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inch;

impl PhysicalUnit for Inch {
    const TO_POINTS: f64 = 72.0;
}

/// Length representation for [`UserSpace`] and physical space [`PhysicalUnit`].
///
/// It also guarantees that the containing float number is finite.
#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct Length<U> {
    value: f64,
    _unit: PhantomData<U>,
}

impl<U> Clone for Length<U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> Copy for Length<U> {}

impl<U> TryFrom<f64> for Length<U> {
    type Error = Error;
    fn try_from(value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(Error::NonFinite);
        }
        Ok(Self {
            value,
            _unit: PhantomData,
        })
    }
}

impl<U> Length<U> {
    /// Length of 0.0
    pub const ZERO: Self = Self::from_raw(0.0);
    /// Length of 1.0
    pub const ONE: Self = Self::from_raw(1.0);

    pub(crate) const fn from_raw(value: f64) -> Self {
        Self {
            value,
            _unit: PhantomData,
        }
    }

    /// Returns the raw value.
    #[inline]
    #[must_use]
    pub fn get(&self) -> f64 {
        self.value
    }
}

macro_rules! length_from {
      ($from:ident => $($to:ident),+) => {$(
          impl From<Length<$from>> for Length<$to> {
              fn from(l: Length<$from>) -> Self {
                  Length::from_raw(l.get() * <$from as PhysicalUnit>::TO_POINTS / <$to as PhysicalUnit>::TO_POINTS)
              }
          }
      )+};
  }
length_from!(Mm => Pt);
length_from!(Pt => Mm);

impl<U: PhysicalUnit> Length<U> {
    /// Converts from physical unit to the user space.
    pub fn to_user(self, uu: UserUnit) -> Length<UserSpace> {
        Length::from_raw(self.value * U::TO_POINTS / uu.get())
    }
}

impl Length<UserSpace> {
    /// Converts from user space to the physical unit.
    pub fn to_physical<U: PhysicalUnit>(self, uu: UserUnit) -> Length<U> {
        Length::from_raw(self.value * uu.get() / U::TO_POINTS)
    }
}

impl<U> Add for Length<U> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Length::from_raw(self.value + rhs.value)
    }
}

impl<U> Sub for Length<U> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Length::from_raw(self.value - rhs.value)
    }
}

/// Represents the pt unit in [`UserSpace`].
#[derive(Debug, Clone, Copy)]
pub struct UserUnit(pub(crate) f64);

impl UserUnit {
    /// Returns the raw value.
    #[inline]
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn mm_pt_conversion_identity_1mm() -> Result<()> {
        let value = 1.0;
        let length_mm = Length::<Mm>::try_from(value)?;
        let length_pt: Length<Pt> = length_mm.into();
        let length_mm_from_pt: Length<Mm> = length_pt.into();

        assert_eq!(length_mm, length_mm_from_pt);
        Ok(())
    }

    #[test]
    fn mm_pt_non_finite() {
        [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
            .iter()
            .for_each(|&value| {
                let mm = Length::<Mm>::try_from(value);
                assert!(matches!(mm, Err(Error::NonFinite)));
                let pt = Length::<Pt>::try_from(value);
                assert!(matches!(pt, Err(Error::NonFinite)));
            });
    }

    #[test]
    fn mm_pt_conversion() -> Result<()> {
        let mm = Length::<Mm>::try_from(4.5)?;

        let pt: Length<Pt> = mm.into();
        // 4.5 * 72.0 / 25.4
        let pt_test = Length::<Pt>::try_from(12.755905511811024)?;
        assert!((pt.get() - pt_test.get()).abs() < EPS);

        let mm_test: Length<Mm> = pt_test.into();
        assert!((mm.get() - mm_test.get()).abs() < EPS);
        Ok(())
    }

    #[test]
    fn length_ops() -> Result<()> {
        let len: Length<Mm> = 1.0.try_into()?;
        let len_rhs = len;
        let len_test = Length::try_from(2.0)?;
        assert_eq!(len + len_rhs, len_test);
        Ok(())
    }
}
