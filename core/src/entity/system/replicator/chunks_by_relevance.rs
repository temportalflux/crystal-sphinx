use crate::entity::system::replicator::relevancy::{AxisAlignedBoundingBox, Relevance};
use engine::math::nalgebra::Point3;
use std::collections::HashSet;

pub struct ChunksByRelevance {
	unique_set: HashSet<Point3<i64>>,
	// Vec sorted by relevance, where the start is the least relevant and the end is the most relevant.
	sorted: Vec<Point3<i64>>,
}

impl ChunksByRelevance {
	pub fn new() -> Self {
		Self {
			unique_set: HashSet::new(),
			sorted: Vec::new(),
		}
	}

	pub fn len(&self) -> usize {
		self.unique_set.len()
	}

	fn cmp_relevance(
		a: &Point3<i64>,
		b: &Point3<i64>,
		relevance: &Relevance,
	) -> std::cmp::Ordering {
		let a_dist = relevance.min_dist_to_relevance(&a);
		let b_dist = relevance.min_dist_to_relevance(&b);
		b_dist
			.partial_cmp(&a_dist)
			.unwrap_or(std::cmp::Ordering::Equal)
	}

	#[profiling::function]
	pub fn retain_and_sort_by(&mut self, relevance: &Relevance) {
		self.retain(relevance);
		self.sorted
			.sort_by(|a, b| Self::cmp_relevance(a, b, relevance));
	}

	#[profiling::function]
	fn retain(&mut self, relevance: &Relevance) {
		self.unique_set
			.retain(|coord| relevance.is_relevant(&coord));
		self.sorted.retain(|coord| relevance.is_relevant(&coord));
	}

	#[profiling::function]
	pub fn insert_cuboids(
		&mut self,
		cuboids: HashSet<AxisAlignedBoundingBox>,
		relevance: &Relevance,
	) {
		for cuboid in cuboids.into_iter() {
			let cuboid_coords: HashSet<Point3<i64>> = cuboid.into();
			for coord in cuboid_coords {
				if let Some(idx) = self.find_insertion_point(&coord, relevance) {
					self.insert(idx, coord);
				}
			}
		}
	}

	#[profiling::function]
	pub fn find_insertion_point(
		&self,
		coord: &Point3<i64>,
		relevance: &Relevance,
	) -> Option<usize> {
		if self.unique_set.contains(coord) {
			return None;
		}
		let search_res = self
			.sorted
			.binary_search_by(|a| Self::cmp_relevance(a, &coord, relevance));
		Some(match search_res {
			Ok(idx) => idx,
			Err(idx) => idx,
		})
	}

	#[profiling::function]
	pub fn insert(&mut self, idx: usize, coord: Point3<i64>) {
		if self.unique_set.insert(coord) {
			self.sorted.insert(idx, coord);
		}
	}

	#[profiling::function]
	pub fn pop_front(&mut self) -> Option<Point3<i64>> {
		match self.sorted.pop() {
			Some(coord) => {
				self.unique_set.remove(&coord);
				Some(coord)
			}
			None => None,
		}
	}

	pub fn into_sorted(self) -> Vec<Point3<i64>> {
		self.sorted
	}
}
