//! Affine transformation matrix and methods

use lopdf::Object;

use crate::codec::TryFromObject;
use crate::content::TryFromOperands;
use crate::dict::DictKey;
use crate::geometry::{Point, Rect};
use crate::unit::UserSpace;
use crate::{Error, Length, ObjectAsF64, Result};

/// Represents an affine transformation matrix.
///
/// ISO 32000-1:2008 8.3.4 Transformation Matrices defines it as follows:
///
/// ```text
/// | a b 0 |
/// | c d 0 |
/// | e f 1 |
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// Horizontal scaling
    pub a: f64,
    /// Horizontal shearing
    pub b: f64,
    /// Vertical shearing
    pub c: f64,
    /// Vertical scaling
    pub d: f64,
    /// Horizontal translation
    pub e: f64,
    /// Vertical translation
    pub f: f64,
}

impl DictKey for Matrix {
    const KEY: &'static [u8] = b"Matrix";
}

impl TryFromOperands for Matrix {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [a, b, c, d, e, f] = operands else {
            return Err(Error::InvalidOperands);
        };

        Ok(Self {
            a: a.as_f64()?,
            b: b.as_f64()?,
            c: c.as_f64()?,
            d: d.as_f64()?,
            e: e.as_f64()?,
            f: f.as_f64()?,
        })
    }
}

impl<'a> TryFromObject<'a> for Matrix {
    fn try_from_object(
        _doc: &'a lopdf::Document,
        _id: Option<lopdf::ObjectId>,
        obj: &'a lopdf::Object,
    ) -> Result<Self> {
        let arr = obj.as_array()?;
        Self::try_from_operands(arr)
    }
}

impl Matrix {
    /// Identity matrix: `[ 1 0 0 1 0 0 ]`
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Returning Other Matrix x Self Matrix.
    pub fn pre_concat(&self, other: &Self) -> Self {
        Self {
            a: other.a * self.a + other.b * self.c,
            b: other.a * self.b + other.b * self.d,
            c: other.c * self.a + other.d * self.c,
            d: other.c * self.b + other.d * self.d,
            e: other.e * self.a + other.f * self.c + self.e,
            f: other.e * self.b + other.f * self.d + self.f,
        }
    }

    /// Returning Self Matrix x Other Matrix.
    pub fn post_concat(&self, other: &Self) -> Self {
        other.pre_concat(self)
    }

    /// Returning Point x Matrix.
    pub fn transform_point(&self, point: &Point<UserSpace>) -> Point<UserSpace> {
        let point_x_pt = point.x.get();
        let point_y_pt = point.y.get();
        Point {
            x: Length::from_raw(self.a * point_x_pt + self.c * point_y_pt + self.e),
            y: Length::from_raw(self.b * point_x_pt + self.d * point_y_pt + self.f),
        }
    }

    /// Returning the bounds of transformed rect.
    pub fn transform_rect_bounds(&self, rect: &Rect<UserSpace>) -> Rect<UserSpace> {
        let corners = [
            rect.lower_left(),
            rect.lower_right(),
            rect.upper_left(),
            rect.upper_right(),
        ];

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for corner in corners {
            let p = self.transform_point(&corner);
            min_x = min_x.min(p.x.get());
            min_y = min_y.min(p.y.get());
            max_x = max_x.max(p.x.get());
            max_y = max_y.max(p.y.get());
        }

        Rect::from_edges(
            Point {
                x: Length::from_raw(min_x),
                y: Length::from_raw(min_y),
            },
            Point {
                x: Length::from_raw(max_x),
                y: Length::from_raw(max_y),
            },
        )
    }

    /// Creates a translation matrix by providing horizontal and vertical translation.
    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_rect_bounds() {
        let rotation = Matrix {
            a: 0.0,
            b: 1.0,
            c: -1.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };

        let bounds = rotation.transform_rect_bounds(&Rect::UNIT);

        let expected = Rect::from_edges(
            Point {
                x: Length::from_raw(-1.0),
                y: Length::from_raw(0.0),
            },
            Point {
                x: Length::from_raw(0.0),
                y: Length::from_raw(1.0),
            },
        );
        assert_eq!(bounds, expected);
    }
}
