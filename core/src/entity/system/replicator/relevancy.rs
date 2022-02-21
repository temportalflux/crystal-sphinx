use crate::server::world::chunk::Chunk;
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

	pub fn get_relevant_chunks(&self) -> HashSet<Point3<i64>> {
		let diameter = 2 * self.1 + 1;
		let mut coordinates = HashSet::with_capacity(diameter.pow(3) as usize);
		let diameter = diameter as i64;
		let centering_offset = Vector3::new(self.1, self.1, self.1).cast::<i64>();
		for y in 0..diameter {
			for x in 0..diameter {
				for z in 0..diameter {
					coordinates.insert(self.0 + Vector3::new(x, y, z) - centering_offset);
				}
			}
		}
		coordinates
	}

	pub fn is_relevant(&self, chunk: &Point3<i64>) -> bool {
		let offset = chunk - self.0;
		return offset.x.abs() as u64 <= self.1
			&& offset.y.abs() as u64 <= self.1
			&& offset.z.abs() as u64 <= self.1;
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

	pub fn get_relevant_chunks(&self) -> HashSet<Point3<i64>> {
		let mut coords = HashSet::new();
		for area in self.0.iter() {
			for coord in area.get_relevant_chunks().into_iter() {
				coords.insert(coord);
			}
		}
		coords
	}

	pub fn is_relevant(&self, chunk: &Point3<i64>) -> bool {
		for area in self.0.iter() {
			if area.is_relevant(&chunk) {
				return true;
			}
		}
		false
	}

	#[profiling::function]
	pub fn difference(&self, other: &Relevance) -> HashSet<Point3<i64>> {
		// For now this is brute force, but there has GOT to be a faster way
		// to calculate the area difference between two sets of cuboid areas.
		let self_chunks = self.get_relevant_chunks();
		let other_chunks = other.get_relevant_chunks();
		self_chunks.difference(&other_chunks).cloned().collect()
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

pub type UpdateSender = async_channel::Sender<Update>;
pub type UpdateReceiver = async_channel::Receiver<Update>;
pub enum Update {
	Entity(Relevance),
	World(WorldUpdate),
}

pub type WorldUpdateSender = async_channel::Sender<WorldUpdate>;
pub type WorldUpdateReceiver = async_channel::Receiver<WorldUpdate>;
pub enum WorldUpdate {
	Relevance(Relevance),
	Chunks(Vec<Weak<RwLock<Chunk>>>),
}
