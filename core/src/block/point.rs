use crate::world::chunk;
use engine::math::nalgebra::{Point3, Vector3};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
	chunk: Point3<i64>,
	offset: Point3<i8>,
}

impl Point {
	pub fn new(chunk: Point3<i64>, offset: Point3<i8>) -> Self {
		let mut point = Self { chunk, offset };
		point.align();
		point
	}

	pub fn chunk(&self) -> &Point3<i64> {
		&self.chunk
	}

	pub fn offset(&self) -> &Point3<i8> {
		&self.offset
	}
}

impl std::fmt::Debug for Point {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}

impl std::fmt::Display for Point {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"<{}'{}, {}'{}, {}'{}>",
			self.chunk.x, self.offset.x, self.chunk.y, self.offset.y, self.chunk.z, self.offset.z,
		)
	}
}

impl std::ops::Add<Vector3<i8>> for Point {
	type Output = Self;
	fn add(mut self, other: Vector3<i8>) -> Self::Output {
		self.offset += other;
		self.align();
		self
	}
}

impl std::ops::Sub<Point> for Point {
	type Output = Self;
	fn sub(self, rhs: Point) -> Self::Output {
		Self::new(
			self.chunk - rhs.chunk.coords,
			self.offset - rhs.offset.coords,
		)
	}
}

impl Point {
	fn align(&mut self) {
		let size = chunk::SIZE_I;
		for i in 0..size.len() {
			let size = size[i] as i8;

			// ex1: offset = 0
			// ex2: offset = 5
			// ex3: offset = -1
			// ex4: offset = 16

			// if offset < 0; -1
			// if offset >= 1; +1
			// ex1: signum = 0
			// ex2: signum = 1
			// ex3: signum = -1
			// ex4: signum = 1
			let signum = self.offset[i].signum() as i64;
			// if offset < 0; +1
			// if offset >= 0; 0
			// ex1: lower_shuffle = max(0, -0) = 0
			// ex2: lower_shuffle = max(0, -1) = 0
			// ex3: lower_shuffle = max(0, +1) = 1
			// ex4: lower_shuffle = max(0, -1) = 0
			let lower_shuffle = 0i64.max(-signum);
			// The amount of chunks that are stored in the offset coord for axis i.
			// For any value < 0, this is always 1 less than the number of chunks to shift (because of negatives).
			// ex1: shift =  abs(0) / 16 = 0
			// ex2: shift =  abs(5) / 16 = 0
			// ex3: shift = abs(-1) / 16 = 0
			// ex4: shift = abs(16) / 16 = 1
			let shift = (self.offset[i].abs() / size) as i64;
			// ex1: shift = shift + lower_shuffle = 0 + 0 = 0
			// ex2: shift = shift + lower_shuffle = 0 + 0 = 0
			// ex3: shift = shift + lower_shuffle = 0 + 1 = 1
			// ex4: shift = shift + lower_shuffle = 1 + 0 = 1
			let shift = shift + lower_shuffle;
			// The amount of chunks to shift on axis i.
			// ex1: chunk_shift =  0 * 0 =  0
			// ex2: chunk_shift =  1 * 0 =  0
			// ex3: chunk_shift = -1 * 1 = -1
			// ex4: chunk_shift =  1 * 1 = +1
			let chunk_shift = signum * shift;
			// The amount to remove from `offset` to account for the `chunk_shift`.
			// ex1: offset_shift = (-0 * 0) * 16 =  0
			// ex2: offset_shift = (-1 * 0) * 16 =  0
			// ex3: offset_shift = (+1 * 1) * 16 = +16
			// ex4: offset_shift = (-1 * 1) * 16 = -16
			let offset_shift = (-signum * shift) * (size as i64);

			// offset[i] < 0 || offset[i] >= size
			if shift.abs() > 0 {
				self.chunk[i] += chunk_shift;
				self.offset[i] += offset_shift as i8;
			}

			/*
			// ex1: false
			// ex2: false
			// ex3: true
			// ex4: false
			if self.offset[i] < 0 {
				// ex3: amount = (1/16) + 1 = 1
				let amount = (self.offset[i].abs() / size) + 1;
				// ex3: chunk[i] -= 1
				self.chunk[i] -= amount as i64;
				// ex3: offset[i] += 1 * 16
				//			offset[i] = 16 - 16 = 0
				self.offset[i] += amount * size;
			}
			// ex1: false
			// ex2: false
			// ex3: false
			// ex4: true
			if self.offset[i] >= size {
				// ex4: amount = 16 / 16 = 1
				let amount = self.offset[i].abs() / size;
				// ex4: chunk[i] += 1
				self.chunk[i] += amount as i64;
				// ex4: offset[i] = 16 - (1 * 16) = 0
				self.offset[i] -= amount * size;
			}
			*/
		}
	}
}

#[cfg(test)]
mod block_point {
	use super::*;

	#[test]
	fn add_vector3_origin_x1() {
		let point = Point::new(Point3::new(0, 0, 0), Point3::new(0, 0, 0));
		let change = Vector3::new(1, 0, 0);
		assert_eq!(
			point + change,
			Point::new(Point3::new(0, 0, 0), Point3::new(1, 0, 0))
		);
	}

	#[test]
	fn add_vector3_origin_y5() {
		let point = Point::new(Point3::new(0, 0, 0), Point3::new(0, 0, 0));
		let change = Vector3::new(0, 5, 0);
		let expected = Point::new(Point3::new(0, 0, 0), Point3::new(0, 5, 0));
		assert_eq!(point + change, expected);
	}

	#[test]
	fn add_vector3_z10_z6_chunk_z1() {
		let point = Point::new(Point3::new(0, 0, 0), Point3::new(0, 0, 10));
		let change = Vector3::new(0, 0, 6);
		let expected = Point::new(Point3::new(0, 0, 1), Point3::new(0, 0, 0));
		assert_eq!(point + change, expected);
	}

	#[test]
	fn add_vector3_x0y0z0_xm1() {
		let point = Point::new(Point3::new(0, 0, 0), Point3::new(0, 0, 0));
		let change = Vector3::new(-1, 0, 0);
		let expected = Point::new(Point3::new(-1, 0, 0), Point3::new(15, 0, 0));
		assert_eq!(point + change, expected);
	}

	#[test]
	fn subtract_point() {
		let point = Point::new(Point3::new(0, 5, 3), Point3::new(1, 7, 2));
		let change = Point::new(Point3::new(1, 0, 0), Point3::new(0, 7, 3));
		let expected = Point::new(Point3::new(-1, 5, 2), Point3::new(1, 0, 15));
		assert_eq!(point - change, expected);
	}
}
