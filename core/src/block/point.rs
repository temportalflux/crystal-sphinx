use crate::common::world;

pub type Point = world::Point<i8>;

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
