use crate::server::world::chunk::Chunk;
use engine::channels::future::{Receiver, Sender};
use engine::math::nalgebra::{Point3, Vector3};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashSet,
	sync::{RwLock, Weak},
};

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Area(Point3<i64>, u64);

impl std::fmt::Debug for Area {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "<{}, {}, {}>r{}", self.0.x, self.0.y, self.0.z, self.1)
	}
}

impl Area {
	pub fn new(point: Point3<i64>, radius: u64) -> Self {
		Self(point, radius)
	}

	pub fn is_relevant(&self, chunk: &Point3<i64>) -> bool {
		let offset = chunk - self.0;
		return offset.x.abs() as u64 <= self.1
			&& offset.y.abs() as u64 <= self.1
			&& offset.z.abs() as u64 <= self.1;
	}

	pub fn min_dist_to_relevance(&self, chunk: &Point3<i64>) -> f64 {
		let offset = chunk - self.0;
		offset.cast::<f64>().magnitude()
	}
}

#[derive(Default)]
pub struct PairedRelevance {
	pub chunk: Relevance,
	pub entity: Relevance,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Relevance(Vec<Area>);

impl std::fmt::Debug for Relevance {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Relevance({:?})", self.0)
	}
}

impl Relevance {
	pub fn push(&mut self, area: Area) {
		self.0.push(area);
	}

	#[profiling::function]
	fn as_cuboids(&self) -> HashSet<AxisAlignedBoundingBox> {
		let mut cuboids = HashSet::new();
		for area in self.0.iter() {
			let radius = area.1 as i64;
			let radius_vec = Vector3::new(radius, radius, radius);
			cuboids.insert(AxisAlignedBoundingBox {
				min: area.0 - radius_vec,
				max: area.0 + radius_vec,
			});
		}
		cuboids
	}

	pub fn is_relevant(&self, chunk: &Point3<i64>) -> bool {
		for area in self.0.iter() {
			if area.is_relevant(&chunk) {
				return true;
			}
		}
		false
	}

	pub fn min_dist_to_relevance(&self, chunk: &Point3<i64>) -> f64 {
		let mut dist = f64::MAX;
		for area in self.0.iter() {
			let d = area.min_dist_to_relevance(&chunk);
			if d < dist {
				dist = d;
			}
		}
		dist
	}

	#[profiling::function]
	pub fn difference(&self, other: &Relevance) -> HashSet<AxisAlignedBoundingBox> {
		// M1: This has terrible performance: like 20ms+ for a diff between 2 radial areas
		// with a radius of 6 (because each would have a cuboid area of (2r+1)^3 ≅ 2200 coordinates).
		/*
		let self_chunks = self.get_relevant_chunks();
		let other_chunks = other.get_relevant_chunks();
		self_chunks.difference(&other_chunks).cloned().collect()
		*/

		// M2: This is still bad because its a full iteration over 2200 coordinates, its just not as bad as 2200^2.
		// It still results in about 10ms+ per diff.
		/*
		let mut self_chunks = self.get_relevant_chunks();
		self_chunks.retain(|coord| {
			!other.is_relevant(&coord)
		});
		self_chunks
		*/

		// M3
		let mut cuboids = self.as_cuboids();
		for other_cuboid in other.as_cuboids().into_iter() {
			let mut resulting_cuboids = HashSet::with_capacity(cuboids.len());
			for cuboid in cuboids.into_iter() {
				if let Some(not_in_other) = cuboid.difference(&other_cuboid) {
					for cuboid in not_in_other.into_iter() {
						resulting_cuboids.insert(cuboid);
					}
				}
			}
			cuboids = resulting_cuboids;
		}
		cuboids
	}

	/// Returns the minimum significant distance squared by
	/// comparing the provided point against the origin of each area in the group.
	pub fn min_sig_dist_sq(&self, point: &Point3<i64>) -> f32 {
		self.0
			.iter()
			.map(|area| (point - area.0).cast::<f32>().magnitude_squared())
			.fold(f32::INFINITY, |a1, a2| a1.min(a2))
	}

	#[profiling::function]
	pub fn sort_vec_by_sig_dist(&self, points: &mut Vec<Point3<i64>>) {
		points.sort_by(|a, b| {
			let a_dist = self.min_sig_dist_sq(&a);
			let b_dist = self.min_sig_dist_sq(&b);
			a_dist.partial_cmp(&b_dist).unwrap()
		});
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AxisAlignedBoundingBox {
	/// Inclusive minima of each axis
	min: Point3<i64>,
	/// Exclusive maxima of each axis
	max: Point3<i64>,
}

impl std::fmt::Debug for AxisAlignedBoundingBox {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}

impl std::fmt::Display for AxisAlignedBoundingBox {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"AABB(<{}, {}, {}> -> <{}, {}, {}>)",
			self.min.x, self.min.y, self.min.z, self.max.x, self.max.y, self.max.z
		)
	}
}

impl AxisAlignedBoundingBox {
	/// AABBxAABB intersection test
	/// `<https://developer.mozilla.org/en-US/docs/Games/Techniques/3D_collision_detection#aabb_vs._aabb>`
	fn intersects(&self, other: &Self) -> bool {
		let x = self.min.x < other.max.x && other.min.x < self.max.x;
		let y = self.min.y < other.max.y && other.min.y < self.max.y;
		let z = self.min.z < other.max.z && other.min.z < self.max.z;
		return x && y && z;
	}

	fn overlap(&self, other: &Self) -> Option<Self> {
		if !self.intersects(other) {
			return None;
		}

		// the component-wise maximum of the two minima
		let min = self.min.sup(&other.min);
		// the component-wise minimum of the two maxima
		let max = self.max.inf(&other.max);

		Some(Self { min, max })
	}

	/// Performs and [`overlap`](Self::overlap) test and returns a set
	/// of cuboids representing the area of self without the overlap.
	/// If the provided cuboid does not intersect with self, the cuboid itself is returned.
	/// If the cuboids are identical, None is returned.
	fn difference(&self, other: &Self) -> Option<HashSet<Self>> {
		let overlap = match self.overlap(&other) {
			Some(overlap) => overlap,
			None => return Some(HashSet::from([*self])),
		};

		// This is basically Binary-Space-Partitioning (BSP) but just for cuboids.
		// The goal here is to split the cuboid `self` based on the bounds of `overlap`,
		// and only return the cuboids that are not equal to `overlap`.

		let lower_mid = self.min.sup(&overlap.min);
		let upper_mid = self.max.inf(&overlap.max);
		if lower_mid == self.min && upper_mid == self.max {
			return None;
		}

		let mut cuboids = Self::subdivide(vec![&self.min, &lower_mid, &upper_mid, &self.max]);
		let removed = cuboids.remove(&overlap);
		assert!(removed);

		Some(cuboids)
	}

	fn subdivide(bounds: Vec<&Point3<i64>>) -> HashSet<Self> {
		let row_len = bounds.len() - 1;
		let mut cuboids = Vec::with_capacity(row_len.pow(3));
		for i_y in 0..row_len {
			if bounds[i_y + 0].y == bounds[i_y + 1].y {
				continue;
			}
			for i_z in 0..row_len {
				if bounds[i_z + 0].z == bounds[i_z + 1].z {
					continue;
				}
				for i_x in 0..row_len {
					if bounds[i_x + 0].x == bounds[i_x + 1].x {
						continue;
					}
					cuboids.push(Self {
						min: Point3::new(bounds[i_x + 0].x, bounds[i_y + 0].y, bounds[i_z + 0].z),
						max: Point3::new(bounds[i_x + 1].x, bounds[i_y + 1].y, bounds[i_z + 1].z),
					});
				}
			}
		}
		cuboids.into_iter().collect()
	}
}

impl Into<HashSet<Point3<i64>>> for AxisAlignedBoundingBox {
	fn into(self) -> HashSet<Point3<i64>> {
		let mut coords = HashSet::new();
		for y in self.min.y..self.max.y {
			for z in self.min.z..self.max.z {
				for x in self.min.x..self.max.x {
					coords.insert(Point3::new(x, y, z));
				}
			}
		}
		coords
	}
}

#[cfg(test)]
mod axis_aligned_bounding_box {
	use super::AxisAlignedBoundingBox as AABB;
	use engine::math::nalgebra::Point3;
	use std::collections::HashSet;

	#[test]
	fn intersects_none() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(1, 1, 1),
		};
		let b = AABB {
			min: Point3::new(1, 1, 1),
			max: Point3::new(2, 2, 2),
		};
		assert_eq!(a.intersects(&b), false);
	}

	#[test]
	fn intersects_lhs() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(2, 2, 2),
		};
		let b = AABB {
			min: Point3::new(1, 1, 1),
			max: Point3::new(3, 3, 3),
		};
		assert_eq!(a.intersects(&b), true);
	}

	#[test]
	fn intersects_rhs() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(2, 2, 2),
		};
		let b = AABB {
			min: Point3::new(1, 1, 1),
			max: Point3::new(3, 3, 3),
		};
		assert_eq!(b.intersects(&a), true);
	}

	#[test]
	fn intersects_equal() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(2, 2, 2),
		};
		assert_eq!(a.intersects(&a), true);
	}

	#[test]
	fn overlap_none() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(1, 1, 1),
		};
		let b = AABB {
			min: Point3::new(1, 1, 1),
			max: Point3::new(2, 2, 2),
		};
		assert_eq!(a.overlap(&b), None);
	}

	#[test]
	fn overlap_lower() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(6, 6, 6),
		};
		let b = AABB {
			min: Point3::new(4, 4, 4),
			max: Point3::new(7, 7, 7),
		};
		assert_eq!(
			a.overlap(&b),
			Some(AABB {
				min: Point3::new(4, 4, 4),
				max: Point3::new(6, 6, 6)
			})
		);
	}

	#[test]
	fn overlap_upper() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(6, 6, 6),
		};
		let b = AABB {
			min: Point3::new(4, 4, 4),
			max: Point3::new(7, 7, 7),
		};
		assert_eq!(
			b.overlap(&a),
			Some(AABB {
				min: Point3::new(4, 4, 4),
				max: Point3::new(6, 6, 6)
			})
		);
	}

	#[test]
	fn overlap_equal() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(2, 2, 2),
		};
		assert_eq!(a.overlap(&a), Some(a));
	}

	#[test]
	fn difference_none() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(1, 1, 1),
		};
		let b = AABB {
			min: Point3::new(1, 1, 1),
			max: Point3::new(2, 2, 2),
		};
		assert_eq!(a.difference(&b), Some(HashSet::from([a])));
	}

	#[test]
	fn difference_equal() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(2, 2, 2),
		};
		assert_eq!(a.difference(&a), None);
	}

	#[test]
	fn difference_partial() {
		let a = AABB {
			min: Point3::new(0, 0, 0),
			max: Point3::new(6, 6, 6),
		};
		let b = AABB {
			min: Point3::new(3, 3, 3),
			max: Point3::new(7, 7, 7),
		};
		assert_eq!(
			a.difference(&b),
			Some(HashSet::from([
				AABB {
					min: Point3::new(0, 0, 0),
					max: Point3::new(3, 3, 3)
				},
				AABB {
					min: Point3::new(3, 0, 0),
					max: Point3::new(6, 3, 3)
				},
				AABB {
					min: Point3::new(0, 0, 3),
					max: Point3::new(3, 3, 6)
				},
				AABB {
					min: Point3::new(3, 0, 3),
					max: Point3::new(6, 3, 6)
				},
				AABB {
					min: Point3::new(0, 3, 0),
					max: Point3::new(3, 6, 3)
				},
				AABB {
					min: Point3::new(3, 3, 0),
					max: Point3::new(6, 6, 3)
				},
				AABB {
					min: Point3::new(0, 3, 3),
					max: Point3::new(3, 6, 6)
				},
				//AABB { min: Point3::new(3, 3, 3), max: Point3::new(6, 6, 6) },
			]))
		);
	}

	#[test]
	fn subdivide_one() {
		assert_eq!(
			AABB::subdivide(vec![
				&Point3::new(0, 0, 0),
				&Point3::new(0, 0, 0),
				&Point3::new(6, 6, 6),
				&Point3::new(6, 6, 6),
			]),
			HashSet::from([AABB {
				min: Point3::new(0, 0, 0),
				max: Point3::new(6, 6, 6)
			},])
		);
	}

	#[test]
	fn subdivide_lower_equals_min() {
		use super::AxisAlignedBoundingBox as AABB;
		use engine::math::nalgebra::Point3;
		use std::collections::HashSet;
		assert_eq!(
			AABB::subdivide(vec![
				&Point3::new(0, 0, 0),
				&Point3::new(0, 0, 0),
				&Point3::new(3, 3, 3),
				&Point3::new(6, 6, 6),
			]),
			HashSet::from([
				AABB {
					min: Point3::new(0, 0, 0),
					max: Point3::new(3, 3, 3)
				},
				AABB {
					min: Point3::new(3, 0, 0),
					max: Point3::new(6, 3, 3)
				},
				AABB {
					min: Point3::new(0, 0, 3),
					max: Point3::new(3, 3, 6)
				},
				AABB {
					min: Point3::new(3, 0, 3),
					max: Point3::new(6, 3, 6)
				},
				AABB {
					min: Point3::new(0, 3, 0),
					max: Point3::new(3, 6, 3)
				},
				AABB {
					min: Point3::new(3, 3, 0),
					max: Point3::new(6, 6, 3)
				},
				AABB {
					min: Point3::new(0, 3, 3),
					max: Point3::new(3, 6, 6)
				},
				AABB {
					min: Point3::new(3, 3, 3),
					max: Point3::new(6, 6, 6)
				},
			])
		);
	}

	#[test]
	fn subdivide_upper_equals_max() {
		use super::AxisAlignedBoundingBox as AABB;
		use engine::math::nalgebra::Point3;
		use std::collections::HashSet;
		assert_eq!(
			AABB::subdivide(vec![
				&Point3::new(0, 0, 0),
				&Point3::new(1, 1, 1),
				&Point3::new(3, 3, 3),
				&Point3::new(3, 3, 3),
			]),
			HashSet::from([
				AABB {
					min: Point3::new(0, 0, 0),
					max: Point3::new(1, 1, 1)
				},
				AABB {
					min: Point3::new(1, 0, 0),
					max: Point3::new(3, 1, 1)
				},
				AABB {
					min: Point3::new(0, 0, 1),
					max: Point3::new(1, 1, 3)
				},
				AABB {
					min: Point3::new(1, 0, 1),
					max: Point3::new(3, 1, 3)
				},
				AABB {
					min: Point3::new(0, 1, 0),
					max: Point3::new(1, 3, 1)
				},
				AABB {
					min: Point3::new(1, 1, 0),
					max: Point3::new(3, 3, 1)
				},
				AABB {
					min: Point3::new(0, 1, 1),
					max: Point3::new(1, 3, 3)
				},
				AABB {
					min: Point3::new(1, 1, 1),
					max: Point3::new(3, 3, 3)
				},
			])
		);
	}

	#[test]
	fn subdivide_four() {
		use super::AxisAlignedBoundingBox as AABB;
		use engine::math::nalgebra::Point3;
		use std::collections::HashSet;
		assert_eq!(
			AABB::subdivide(vec![
				&Point3::new(0, 0, 0),
				&Point3::new(1, 1, 1),
				&Point3::new(3, 3, 3),
				&Point3::new(6, 6, 6),
			]),
			HashSet::from([
				AABB {
					min: Point3::new(0, 0, 0),
					max: Point3::new(1, 1, 1)
				},
				AABB {
					min: Point3::new(1, 0, 0),
					max: Point3::new(3, 1, 1)
				},
				AABB {
					min: Point3::new(3, 0, 0),
					max: Point3::new(6, 1, 1)
				},
				AABB {
					min: Point3::new(0, 0, 1),
					max: Point3::new(1, 1, 3)
				},
				AABB {
					min: Point3::new(1, 0, 1),
					max: Point3::new(3, 1, 3)
				},
				AABB {
					min: Point3::new(3, 0, 1),
					max: Point3::new(6, 1, 3)
				},
				AABB {
					min: Point3::new(0, 0, 3),
					max: Point3::new(1, 1, 6)
				},
				AABB {
					min: Point3::new(1, 0, 3),
					max: Point3::new(3, 1, 6)
				},
				AABB {
					min: Point3::new(3, 0, 3),
					max: Point3::new(6, 1, 6)
				},
				AABB {
					min: Point3::new(0, 1, 0),
					max: Point3::new(1, 3, 1)
				},
				AABB {
					min: Point3::new(1, 1, 0),
					max: Point3::new(3, 3, 1)
				},
				AABB {
					min: Point3::new(3, 1, 0),
					max: Point3::new(6, 3, 1)
				},
				AABB {
					min: Point3::new(0, 1, 1),
					max: Point3::new(1, 3, 3)
				},
				AABB {
					min: Point3::new(1, 1, 1),
					max: Point3::new(3, 3, 3)
				},
				AABB {
					min: Point3::new(3, 1, 1),
					max: Point3::new(6, 3, 3)
				},
				AABB {
					min: Point3::new(0, 1, 3),
					max: Point3::new(1, 3, 6)
				},
				AABB {
					min: Point3::new(1, 1, 3),
					max: Point3::new(3, 3, 6)
				},
				AABB {
					min: Point3::new(3, 1, 3),
					max: Point3::new(6, 3, 6)
				},
				AABB {
					min: Point3::new(0, 3, 0),
					max: Point3::new(1, 6, 1)
				},
				AABB {
					min: Point3::new(1, 3, 0),
					max: Point3::new(3, 6, 1)
				},
				AABB {
					min: Point3::new(3, 3, 0),
					max: Point3::new(6, 6, 1)
				},
				AABB {
					min: Point3::new(0, 3, 1),
					max: Point3::new(1, 6, 3)
				},
				AABB {
					min: Point3::new(1, 3, 1),
					max: Point3::new(3, 6, 3)
				},
				AABB {
					min: Point3::new(3, 3, 1),
					max: Point3::new(6, 6, 3)
				},
				AABB {
					min: Point3::new(0, 3, 3),
					max: Point3::new(1, 6, 6)
				},
				AABB {
					min: Point3::new(1, 3, 3),
					max: Point3::new(3, 6, 6)
				},
				AABB {
					min: Point3::new(3, 3, 3),
					max: Point3::new(6, 6, 6)
				},
			])
		);
	}
}

pub type UpdateSender = Sender<Update>;
pub type UpdateReceiver = Receiver<Update>;
pub enum Update {
	Entity(Relevance),
	World(WorldUpdate),
}

pub type WorldUpdateSender = Sender<WorldUpdate>;
pub type WorldUpdateReceiver = Receiver<WorldUpdate>;
pub enum WorldUpdate {
	Relevance(Relevance),
	Chunks(Vec<Weak<RwLock<Chunk>>>),
}
