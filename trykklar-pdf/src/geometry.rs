//! Geometry related types.

use crate::codec::TryFromObject;
use crate::content::TryFromOperands;
use crate::dict::DictKey;
use crate::matrix::Matrix;
use crate::unit::{UserSpace, UserUnit};
use crate::{Error, Length, ObjectAsF64, PhysicalUnit, Result};
use lopdf::Object;
use std::f64;
use std::marker::PhantomData;
use std::ops::{Add, Deref, DerefMut, Sub};
use std::sync::Arc;

/// Current path containing the array of parsed [`PathElement`] in [`UserSpace`], used in the
/// [`crate::ContentWalker`] to track path operations.
#[derive(Debug, Clone, Default)]
pub struct CurrentPath(pub(crate) Arc<Vec<Result<PathElement<UserSpace>>>>);

impl Deref for CurrentPath {
    type Target = Vec<Result<PathElement<UserSpace>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CurrentPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

/// Path Construction Operators
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// As in many places in the crate, if a text is cited, it refers to the last mentioned reference.
#[derive(Debug, Clone)]
pub enum PathElement<U> {
    /// Move current point to.
    MoveTo(MoveTo<U>),
    /// Append a straight line from current point.
    LineTo(LineTo<U>),
    /// Append Bezier Curve to current path, extend from current point to third and using first and
    /// second point as bezier control points. New current point is the third.
    CurveTo(CurveTo<U>),
    /// Append Bezier Curve to current path, extend from current point to third and using current
    /// and second point as bezier control points. New current point is the third.
    CurveToControlCurrentTwo(CurveToControlCurrentTwo<U>),
    /// Append Bezier Curve to current path, extend from current point to third and using first
    /// and third point as bezier control points. New current point is the third.
    CurveToControlOneThree(CurveToControlOneThree<U>),
    /// > Close the current subpath by appending a straight line segment  from the current point to
    /// > the starting point the subpath. Close the current subpath by appending a straight line
    /// > segment from the current point to the starting point of the subpath. If the current
    /// > subpath is already closed, h shall do nothing.
    ///
    /// > This operator terminates the current subpath.
    /// > Appending another segment to the current path shall begin a new subpath, even if the new
    /// > segment begins at the endpoint reached by the h operation.
    Close,
    /// > Append a rectangle to the current path as a complete subpath, with lower-left corner (x,
    /// > y) and dimensions width and height in user space. The operation
    ///
    /// > `x y width height re`
    ///
    /// > is equivalent to
    ///
    /// > ```text
    /// > x y m
    /// > ( x + width ) y l
    /// > ( x + width ) ( y + height ) l
    /// > x ( y + height ) l
    /// > h
    /// > ```
    Rect(Rect<U>),
}

impl PathElement<UserSpace> {
    /// Transform every point in an element by a [`Matrix`].
    pub(crate) fn transform(&self, m: &Matrix) -> Self {
        let map = |point: &Point<UserSpace>| m.transform_point(point);
        match self {
            Self::MoveTo(e) => Self::MoveTo(MoveTo(map(&e.get()))),
            Self::LineTo(e) => Self::LineTo(LineTo(map(&e.get()))),
            Self::CurveTo(e) => {
                let (a, b, c) = e.get();
                Self::CurveTo(CurveTo(map(&a), map(&b), map(&c)))
            }
            Self::CurveToControlCurrentTwo(e) => {
                let (a, b) = e.get();
                Self::CurveToControlCurrentTwo(CurveToControlCurrentTwo(map(&a), map(&b)))
            }
            Self::CurveToControlOneThree(e) => {
                let (a, b) = e.get();
                Self::CurveToControlOneThree(CurveToControlOneThree(map(&a), map(&b)))
            }
            Self::Rect(r) => Self::Rect(m.transform_rect_bounds(r)),
            Self::Close => Self::Close,
        }
    }
}

impl<U> PathElement<U> {
    /// Utility for_each implementation for each point in an element.
    pub fn for_each_point(&self, mut f: impl FnMut(Point<U>)) {
        match self {
            Self::MoveTo(p) => f(p.get()),
            Self::LineTo(p) => f(p.get()),
            Self::CurveTo(c) => {
                let (p1, p2, p3) = c.get();
                f(p1);
                f(p2);
                f(p3);
            }
            Self::CurveToControlCurrentTwo(c) => {
                let (p1, p2) = c.get();
                f(p1);
                f(p2);
            }
            Self::CurveToControlOneThree(c) => {
                let (p1, p2) = c.get();
                f(p1);
                f(p2);
            }
            Self::Rect(r) => {
                f(r.lower_left());
                f(r.upper_left());
                f(r.upper_right());
                f(r.lower_right());
            }
            Self::Close => {}
        }
    }
}

/// `m` Operator, `x y` Operands
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// > Begin a new subpath by moving the current point to coordinates (x, y), omitting any connecting
/// > line segment. If the previous path construction operator in the current path was also m, the
/// > new m overrides it; no vestige of the previous m operation remains in the path.
#[derive(Debug, Clone, Copy)]
pub struct MoveTo<U>(Point<U>);

impl<U> MoveTo<U> {
    /// Get the point.
    pub fn get(&self) -> Point<U> {
        self.0
    }
}

impl<User> TryFromOperands for MoveTo<User> {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [x_obj, y_obj] = operands else {
            return Err(Error::InvalidOperands);
        };
        let x = x_obj.as_f64()?;
        let y = y_obj.as_f64()?;
        Ok(Self(Point {
            x: x.try_into()?,
            y: y.try_into()?,
        }))
    }
}

/// `l` (lowercase L) Operator, `x y` Operands
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// > Append a straight line segment from the current point to the point (x, y). The new current
/// > point shall be (x, y).
#[derive(Debug, Clone)]
pub struct LineTo<U>(Point<U>);

impl<U> LineTo<U> {
    /// Get the point.
    pub fn get(&self) -> Point<U> {
        self.0
    }
}

impl<User> TryFromOperands for LineTo<User> {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [x_obj, y_obj] = operands else {
            return Err(Error::InvalidOperands);
        };
        let x = x_obj.as_f64()?;
        let y = y_obj.as_f64()?;
        Ok(Self(Point {
            x: x.try_into()?,
            y: y.try_into()?,
        }))
    }
}

/// `c` Operator, `x1 y1 x2 y2 x3 y3` Operands
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// > Append a cubic Bézier curve to the current path. The curve shall extend from the current point
/// > to the point (x3 , y3 ), using (x1 , y1 ) and (x2 , y2 ) as the Bézier control points (see
/// > 8.5.2.2, "Cubic Bézier Curves"). The new current point shall be (x3 , y3 ).
#[derive(Debug, Clone)]
pub struct CurveTo<U>(Point<U>, Point<U>, Point<U>);

impl<U> CurveTo<U> {
    /// Get the three points.
    pub fn get(&self) -> (Point<U>, Point<U>, Point<U>) {
        (self.0, self.1, self.2)
    }
}

impl<User> TryFromOperands for CurveTo<User> {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [x1_obj, y1_obj, x2_obj, y2_obj, x3_obj, y3_obj] = operands else {
            return Err(Error::InvalidOperands);
        };
        let p1 = Point {
            x: x1_obj.as_f64()?.try_into()?,
            y: y1_obj.as_f64()?.try_into()?,
        };
        let p2 = Point {
            x: x2_obj.as_f64()?.try_into()?,
            y: y2_obj.as_f64()?.try_into()?,
        };
        let p3 = Point {
            x: x3_obj.as_f64()?.try_into()?,
            y: y3_obj.as_f64()?.try_into()?,
        };
        Ok(Self(p1, p2, p3))
    }
}

/// `v` Operator, `x2 y2 x3 y3` Operands
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// > Append a cubic Bézier curve to the current path. The curve shall extend from the current point
/// > to the point (x3 , y3 ), using the current point and (x2 , y2 ) as the Bézier control points
/// > (see 8.5.2.2, "Cubic Bézier Curves"). The new current point shall be (x3 , y3 ).
#[derive(Debug, Clone)]
pub struct CurveToControlCurrentTwo<U>(Point<U>, Point<U>);

impl<U> CurveToControlCurrentTwo<U> {
    /// Get the two points.
    pub fn get(&self) -> (Point<U>, Point<U>) {
        (self.0, self.1)
    }
}

impl<User> TryFromOperands for CurveToControlCurrentTwo<User> {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [x2_obj, y2_obj, x3_obj, y3_obj] = operands else {
            return Err(Error::InvalidOperands);
        };
        let p2 = Point {
            x: x2_obj.as_f64()?.try_into()?,
            y: y2_obj.as_f64()?.try_into()?,
        };
        let p3 = Point {
            x: x3_obj.as_f64()?.try_into()?,
            y: y3_obj.as_f64()?.try_into()?,
        };
        Ok(Self(p2, p3))
    }
}

/// `y` Operator, `x1 y1 x3 y3` Operands
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// > Append a cubic Bézier curve to the current path. The curve shall extend from the current point
/// > to the point (x3 , y3 ), using (x1 , y1 ) and (x3 , y3 ) as the Bézier control points (see
/// > 8.5.2.2, "Cubic Bézier Curves"). The new current point shall be (x3 , y3 ).
#[derive(Debug, Clone)]
pub struct CurveToControlOneThree<U>(Point<U>, Point<U>);

impl<U> CurveToControlOneThree<U> {
    /// Get the two points.
    pub fn get(&self) -> (Point<U>, Point<U>) {
        (self.0, self.1)
    }
}

impl<User> TryFromOperands for CurveToControlOneThree<User> {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [x1_obj, y1_obj, x3_obj, y3_obj] = operands else {
            return Err(Error::InvalidOperands);
        };
        let p1 = Point {
            x: x1_obj.as_f64()?.try_into()?,
            y: y1_obj.as_f64()?.try_into()?,
        };
        let p3 = Point {
            x: x3_obj.as_f64()?.try_into()?,
            y: y3_obj.as_f64()?.try_into()?,
        };
        Ok(Self(p1, p3))
    }
}

/// Size defines a width and height.
#[derive(Debug, PartialEq)]
pub struct Size<U> {
    /// Width.
    pub width: Length<U>,
    /// Height.
    pub height: Length<U>,
}

impl<U> Clone for Size<U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> Copy for Size<U> {}

impl<U> From<Point<U>> for Size<U> {
    fn from(value: Point<U>) -> Self {
        Self {
            width: value.x,
            height: value.y,
        }
    }
}

/// `re` Operator, `x y width height` Operands
///
/// ISO 32000-1:2008 8.5.2 Table 59 - Path Construction Operators
///
/// > Append a rectangle to the current path as a complete subpath, with lower-left corner (x,
/// > y) and dimensions width and height in user space. The operation
///
/// > `x y width height re`
///
/// > is equivalent to
///
/// > ```text
/// > x y m
/// > ( x + width ) y l
/// > ( x + width ) ( y + height ) l
/// > x ( y + height ) l
/// > h
/// > ```
#[derive(Debug, PartialEq)]
pub struct Rect<U> {
    /// Origin.
    pub origin: Point<U>,
    /// Size.
    pub size: Size<U>,
}

impl<U> Clone for Rect<U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> Copy for Rect<U> {}

impl<U> Rect<U> {
    /// Unit rectangle with size 1x1 at origin (0,0).
    pub const UNIT: Self = Self {
        origin: Point::ORIGIN,
        size: Size {
            width: Length::ONE,
            height: Length::ONE,
        },
    };
    /// Returns a new rectangle with all lengths mapped.
    fn map<V>(self, f: impl Fn(Length<U>) -> Length<V>) -> Rect<V> {
        Rect {
            origin: Point {
                x: f(self.origin.x),
                y: f(self.origin.y),
            },
            size: Size {
                width: f(self.size.width),
                height: f(self.size.height),
            },
        }
    }

    /// Creates a new Rect from lower left and upper right point.
    pub fn from_edges(lower_left: Point<U>, upper_right: Point<U>) -> Rect<U> {
        Rect {
            origin: lower_left,
            size: (upper_right - lower_left).into(),
        }
    }

    /// Returns the lower left point.
    pub(crate) fn lower_left(&self) -> Point<U> {
        self.origin
    }

    /// Returns the upper left point.
    pub(crate) fn upper_left(&self) -> Point<U> {
        Point {
            x: self.origin.x,
            y: self.origin.y + self.size.height,
        }
    }

    /// Returns the lower right point.
    pub(crate) fn lower_right(&self) -> Point<U> {
        Point {
            x: self.origin.x + self.size.width,
            y: self.origin.y,
        }
    }

    /// Returns the upper right point.
    pub(crate) fn upper_right(&self) -> Point<U> {
        self.origin + self.size
    }
}

impl Rect<UserSpace> {
    /// Maps to a new rect with a given physical unit.
    pub fn to_physical<U: PhysicalUnit>(self, uu: UserUnit) -> Rect<U> {
        self.map(|length| length.to_physical(uu))
    }

    /// Array of llx, lly, urx, ury
    pub(crate) fn as_box_slice(&self) -> [f64; 4] {
        [
            self.origin.x.get(),
            self.origin.y.get(),
            (self.origin.x + self.size.width).get(),
            (self.origin.y + self.size.height).get(),
        ]
    }
}

impl DictKey for Rect<UserSpace> {
    const KEY: &'static [u8] = b"BBox";
}

impl<U: PhysicalUnit> Rect<U> {
    /// Returns a given rect in a new rect in user space.
    pub fn to_user(self, uu: UserUnit) -> Rect<UserSpace> {
        self.map(|length| length.to_user(uu))
    }
}

impl TryFromObject<'_> for Rect<UserSpace> {
    fn try_from_object(
        doc: &lopdf::Document,
        _id: Option<lopdf::ObjectId>,
        obj: &lopdf::Object,
    ) -> Result<Self> {
        match doc.dereference(obj)?.1 {
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
                Ok(rect)
            }
            _ => Err(Error::InvalidPdfObject("Rectangle value is not an array")),
        }
    }
}

impl TryFromOperands for Rect<UserSpace> {
    fn try_from_operands(operands: &[Object]) -> Result<Self> {
        let [x, y, width, height] = operands else {
            return Err(Error::InvalidOperands);
        };
        let (x, y) = (x.as_f64()?, y.as_f64()?);
        let (width, height) = (width.as_f64()?, height.as_f64()?);
        Self::try_from([x, y, x + width, y + height])
    }
}

impl TryFrom<[f64; 4]> for Rect<UserSpace> {
    type Error = Error;
    /// PDF 32000-1 7.9.5
    fn try_from([llx, lly, urx, ury]: [f64; 4]) -> Result<Self> {
        let width = (llx - urx).abs();
        let height = (lly - ury).abs();
        let x = llx.min(urx);
        let y = lly.min(ury);

        let size = Size {
            width: width.try_into()?,
            height: height.try_into()?,
        };
        let origin = Point {
            x: x.try_into()?,
            y: y.try_into()?,
        };

        Ok(Self { size, origin })
    }
}

/// A point at position x, y.
#[derive(Debug, PartialEq)]
pub struct Point<U> {
    /// Position x.
    pub x: Length<U>,
    /// Position y.
    pub y: Length<U>,
}

impl<U> Clone for Point<U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> Copy for Point<U> {}

impl<U> Point<U> {
    /// Origin point at (0, 0).
    pub const ORIGIN: Self = Self {
        x: Length::ZERO,
        y: Length::ZERO,
    };
}

impl<U> Sub<Point<U>> for Point<U> {
    type Output = Point<U>;
    fn sub(self, rhs: Point<U>) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<U> Add<Size<U>> for Point<U> {
    type Output = Point<U>;
    fn add(self, size: Size<U>) -> Point<U> {
        Point {
            x: self.x + size.width,
            y: self.y + size.height,
        }
    }
}

/// Bounding box
#[derive(Debug, Clone, Copy)]
pub struct BBox<U> {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    _unit: PhantomData<U>,
}

impl<U> Default for BBox<U> {
    fn default() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
            _unit: PhantomData,
        }
    }
}

impl<U> BBox<U> {
    /// Defines an unbounded box at infinity edges.
    pub const UNBOUNDED: Self = Self {
        min_x: f64::NEG_INFINITY,
        min_y: f64::NEG_INFINITY,
        max_x: f64::INFINITY,
        max_y: f64::INFINITY,
        _unit: PhantomData,
    };

    /// Includes with point at x and y position.
    #[inline]
    pub fn include(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    /// Includes with given rectangle.
    pub fn include_rect(&mut self, rect: Rect<U>) {
        self.include(rect.origin.x.get(), rect.origin.y.get());
        self.include(
            (rect.origin.x + rect.size.width).get(),
            (rect.origin.y + rect.size.height).get(),
        );
    }

    /// If the area of the bounding box is zero or negative.
    pub fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    /// Intersects an other bounding box.
    pub fn intersect(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.max(other.min_x),
            min_y: self.min_y.max(other.min_y),
            max_x: self.max_x.min(other.max_x),
            max_y: self.max_y.min(other.max_y),
            _unit: PhantomData,
        }
    }

    /// Returns the rect representation, if the bounding box is not empty. Otherwise `None`.
    pub fn into_rect(self) -> Option<Rect<U>> {
        if self.is_empty() {
            return None;
        }
        let lower_left = Point {
            x: self.min_x.try_into().ok()?,
            y: self.min_y.try_into().ok()?,
        };
        let upper_right = Point {
            x: self.max_x.try_into().ok()?,
            y: self.max_y.try_into().ok()?,
        };
        Some(Rect::from_edges(lower_left, upper_right))
    }
}

impl<U> From<Rect<U>> for BBox<U> {
    fn from(rect: Rect<U>) -> Self {
        let mut bbox = Self::default();
        bbox.include_rect(rect);
        bbox
    }
}
