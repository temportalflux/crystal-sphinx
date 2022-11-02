use crate::common::world::chunk;
use engine::math::nalgebra::{Point3, Vector3};
use num_traits::{AsPrimitive, Euclid};
use serde::{Deserialize, Serialize};
use std::{
	fmt::Debug,
	ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point<V: 'static + Copy + PartialEq + Debug> {
	chunk: Point3<i64>,
	offset: Point3<V>,
}

impl<V> Debug for Point<V>
where
	V: Copy + PartialEq + Debug + std::fmt::Display,
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}

impl<V> std::fmt::Display for Point<V>
where
	V: Copy + PartialEq + Debug + std::fmt::Display,
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"<{:04}`{:.2}, {:04}`{:.2}, {:04}`{:.2}>",
			self.chunk.x, self.offset.x, self.chunk.y, self.offset.y, self.chunk.z, self.offset.z,
		)
	}
}

impl<V> Point<V>
where
	V: Copy + PartialEq + Debug,
{
	pub fn chunk(&self) -> &Point3<i64> {
		&self.chunk
	}

	pub fn offset(&self) -> &Point3<V> {
		&self.offset
	}
}

impl<V> Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
{
	pub fn new(chunk: Point3<i64>, offset: Point3<V>) -> Self {
		let mut point = Self { chunk, offset };
		point.align();
		point
	}

	fn align(&mut self) {
		let size = chunk::SIZE_I;
		for i in 0..size.len() {
			let size: V = size[i].as_();
			self.chunk[i] += self.offset[i].div_euclid(&size).as_();
			self.offset[i] = self.offset[i].rem_euclid(&size);
		}
	}
}

impl<V> From<Vector3<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
{
	fn from(offset: Vector3<V>) -> Self {
		Self::from(Point3::from(offset))
	}
}

impl<V> From<Point3<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
{
	fn from(offset: Point3<V>) -> Self {
		let mut point = Self {
			chunk: Point3::origin(),
			offset,
		};
		point.align();
		point
	}
}

impl<V> Add<Vector3<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AsPrimitive<i64> + Euclid + AddAssign + Add,
	usize: AsPrimitive<V>,
	Point3<V>: AddAssign<Vector3<V>>,
{
	type Output = Self;
	fn add(mut self, other: Vector3<V>) -> Self::Output {
		self += other;
		self
	}
}

impl<V> AddAssign<Vector3<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AsPrimitive<i64> + Euclid + AddAssign + Add,
	usize: AsPrimitive<V>,
	Point3<V>: AddAssign<Vector3<V>>,
{
	fn add_assign(&mut self, rhs: Vector3<V>) {
		self.offset += rhs;
		self.align();
	}
}

impl<V> Sub<Vector3<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AsPrimitive<i64> + Euclid + AddAssign + Add,
	usize: AsPrimitive<V>,
	Point3<V>: SubAssign<Vector3<V>>,
{
	type Output = Self;
	fn sub(mut self, other: Vector3<V>) -> Self::Output {
		self -= other;
		self
	}
}

impl<V> SubAssign<Vector3<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AsPrimitive<i64> + Euclid + AddAssign + Add,
	usize: AsPrimitive<V>,
	Point3<V>: SubAssign<Vector3<V>>,
{
	fn sub_assign(&mut self, rhs: Vector3<V>) {
		self.offset -= rhs;
		self.align();
	}
}

impl<V> Add<Point<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
	Point3<V>: Add<Vector3<V>, Output = Point3<V>>,
{
	type Output = Self;
	fn add(self, rhs: Point<V>) -> Self::Output {
		Self::new(
			self.chunk + rhs.chunk.coords,
			self.offset + rhs.offset.coords,
		)
	}
}

impl<V> AddAssign<Point<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
	Point3<V>: Add<Vector3<V>, Output = Point3<V>>,
{
	fn add_assign(&mut self, rhs: Point<V>) {
		*self = *self + rhs;
	}
}

impl<V> Sub<Point<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
	Point3<V>: Sub<Vector3<V>, Output = Point3<V>>,
{
	type Output = Self;
	fn sub(self, rhs: Point<V>) -> Self::Output {
		Self::new(
			self.chunk - rhs.chunk.coords,
			self.offset - rhs.offset.coords,
		)
	}
}

impl<V> SubAssign<Point<V>> for Point<V>
where
	V: Copy + PartialEq + Debug + AddAssign + AsPrimitive<i64> + Euclid,
	usize: AsPrimitive<V>,
	Point3<V>: Sub<Vector3<V>, Output = Point3<V>>,
{
	fn sub_assign(&mut self, rhs: Point<V>) {
		*self = *self - rhs;
	}
}

impl Point<f32> {
	pub fn as_unified(&self) -> Point3<f32> {
		// NOTE: casting chunk to f32 will loose i64 precision
		self.offset + self.chunk.coords.cast::<f32>().component_mul(&chunk::SIZE)
	}
}

#[cfg(test)]
mod point {
	use super::*;
	use approx::assert_relative_eq;
	use engine::math::nalgebra::point;

	#[test]
	fn new() {
		assert_eq!(
			Point::<i8>::new(point![0, 2, 1], point![5, 1, 3]),
			Point {
				chunk: point![0, 2, 1],
				offset: point![5, 1, 3],
			}
		);
		let point = Point::<f32>::new(point![0, 2, 1], point![5f32, 1f32, 3f32]);
		let expected = Point::<f32> {
			chunk: point![0, 2, 1],
			offset: point![5f32, 1f32, 3f32],
		};
		assert_eq!(point.chunk.x, expected.chunk.x);
		assert_eq!(point.chunk.y, expected.chunk.y);
		assert_eq!(point.chunk.z, expected.chunk.z);
		assert_relative_eq!(point.offset.x, expected.offset.x);
		assert_relative_eq!(point.offset.y, expected.offset.y);
		assert_relative_eq!(point.offset.z, expected.offset.z);
	}

	#[cfg(test)]
	mod align {
		use super::*;

		#[test]
		fn no_change() {
			let mut point = Point {
				chunk: point![0, 0, 0],
				offset: point![1i8, 2i8, 3i8],
			};
			point.align();
			assert_eq!(
				point,
				Point {
					chunk: point![0, 0, 0],
					offset: point![1i8, 2i8, 3i8],
				},
				"i8 no-change alignment"
			);

			let mut point = Point {
				chunk: point![0, 0, 0],
				offset: point![1f32, 2.5f32, 3.8f32],
			};
			point.align();
			let expected = Point {
				chunk: point![0, 0, 0],
				offset: point![1f32, 2.5f32, 3.8f32],
			};
			assert_eq!(point.chunk.x, expected.chunk.x);
			assert_eq!(point.chunk.y, expected.chunk.y);
			assert_eq!(point.chunk.z, expected.chunk.z);
			assert_relative_eq!(point.offset.x, expected.offset.x);
			assert_relative_eq!(point.offset.y, expected.offset.y);
			assert_relative_eq!(point.offset.z, expected.offset.z);
		}

		#[test]
		fn reduce_positive() {
			let mut point = Point {
				chunk: point![0, 0, 0],
				offset: point![17i8, 19i8, 16i8],
			};
			point.align();
			assert_eq!(
				point,
				Point {
					chunk: point![1, 1, 1],
					offset: point![1i8, 3i8, 0i8],
				},
				"i8 reduce positive"
			);

			let mut point = Point {
				chunk: point![0, 0, 0],
				offset: point![17f32, 25.314f32, 34.832f32],
			};
			point.align();
			let expected = Point {
				chunk: point![1, 1, 2],
				offset: point![1f32, 9.314f32, 2.832f32],
			};
			assert_eq!(point.chunk.x, expected.chunk.x);
			assert_eq!(point.chunk.y, expected.chunk.y);
			assert_eq!(point.chunk.z, expected.chunk.z);
			assert_relative_eq!(point.offset.x, expected.offset.x, epsilon = 0.0001);
			assert_relative_eq!(point.offset.y, expected.offset.y, epsilon = 0.0001);
			assert_relative_eq!(point.offset.z, expected.offset.z, epsilon = 0.0001);
		}

		#[test]
		fn reduce_negative() {
			let mut point: Point<i8> = Point {
				chunk: point![0, 0, 0],
				offset: point![-5, -16, -33],
			};
			point.align();
			let expected: Point<i8> = Point {
				chunk: point![-1, -1, -3],
				offset: point![11, 0, 15],
			};
			assert_eq!(point, expected);

			let mut point = Point {
				chunk: point![0, 0, 0],
				offset: point![-17.512f32, -6.125f32, -41.896f32],
			};
			point.align();
			let expected = Point {
				chunk: point![-2, -1, -3],
				offset: point![14.488f32, 9.875f32, 6.104f32],
			};
			assert_eq!(point.chunk.x, expected.chunk.x);
			assert_eq!(point.chunk.y, expected.chunk.y);
			assert_eq!(point.chunk.z, expected.chunk.z);
			assert_relative_eq!(point.offset.x, expected.offset.x, epsilon = 0.0001);
			assert_relative_eq!(point.offset.y, expected.offset.y, epsilon = 0.0001);
			assert_relative_eq!(point.offset.z, expected.offset.z, epsilon = 0.0001);
		}
	}
}
