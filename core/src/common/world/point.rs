use crate::common::world::chunk;
use engine::math::nalgebra::{vector, Point3, Vector3};
use std::cmp::Ordering;
use num_traits::{Signed, One, Zero, Inv};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<V: 'static + Copy + PartialEq + std::fmt::Debug> {
	chunk: Point3<i64>,
	offset: Point3<V>,
}

impl<V> Point<V> {
	pub fn chunk(&self) -> &Point3<i64> {
		&self.chunk
	}

	pub fn offset(&self) -> &Point3<V> {
		&self.offset
	}
}

impl<V> Point<V> where V: Copy + PartialEq + std::fmt::Debug + Ord + Signed + One + Zero + Inv {
	pub fn new(chunk: Point3<i64>, offset: Point3<V>) -> Self {
		let mut point = Self { chunk, offset };
		point.align();
		point
	}

	fn align(&mut self) {
		let size = chunk::SIZE_I.cast::<V>();
		for i in 0..size.len() {
			let size = size[i];

			// if offset < 0; -1
			// if offset >= 1; +1
			let signum = self.offset[i].signum();
			// if offset < 0; +1
			// if offset >= 0; 0
			let lower_shuffle = V::zero().max(-signum);
			// The amount of chunks that are stored in the offset coord for axis i.
			// For any value < 0, this is always 1 less than the number of chunks to shift (because of negatives).
			let shift = self.offset[i].abs() / size;
			let shift = shift + lower_shuffle;
			// The amount of chunks to shift on axis i.
			let chunk_shift = (signum * shift) as i64;
			// The amount to remove from `offset` to account for the `chunk_shift`.
			let offset_shift = (-signum * shift) * size;

			// offset[i] < 0 || offset[i] >= size
			if shift.abs() > 0 {
				self.chunk[i] += chunk_shift;
				self.offset[i] += offset_shift;
			}
		}
	}
}

#[cfg(test)]
mod point {
	use super::*;

	#[cfg(test)]
	mod block_i8 {
		use super::*;

		#[test]
		fn new() {
			assert_eq!(
				Point::new(vector![0, 0, 0], vector![5, 1, 3]),
				Point {
					chunk: vector![0, 0, 0],
					offset: vector![5, 1, 3],
				}
			);
		}

		#[cfg(test)]
		mod align {
			use super::*;

			#[test]
			fn no_change() {
				let mut point: Point<i8> = Point {
					chunk: vector![0, 0, 0],
					offset: vector![1, 2, 3],
				};
				point.align();
				assert_eq!(point, Point<i8> {
					chunk: vector![0, 0, 0],
					offset: vector![1, 2, 3],
				});
			}

			#[test]
			fn over_positive() {
				let mut point: Point<i8> = Point {
					chunk: vector![0, 0, 0],
					offset: vector![17, 19, 16],
				};
				point.align();
				assert_eq!(point, Point<i8> {
					chunk: vector![1, 1, 1],
					offset: vector![1, 3, 0],
				});
			}

		}
	}

	#[cfg(test)]
	mod entity_f32 {
		use super::*;
	}
}
