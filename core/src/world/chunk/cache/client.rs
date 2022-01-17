use crate::{block, world::chunk::Chunk};
use engine::math::nalgebra::Point3;
use multimap::MultiMap;
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, RwLock},
};

pub enum Operation {
	Remove(Point3<i64>),
	Insert(Point3<i64>, Vec<(Point3<usize>, block::LookupId)>),
}

pub type ArcLockClientCache = Arc<RwLock<ClientCache>>;

/// A storage bin for all the chunks which are relevant to the client.
/// Stores strong references until replication packets remove chunks.
pub struct ClientCache {
	loaded_chunks: HashMap<Point3<i64>, Arc<RwLock<Chunk>>>,

	added: MultiMap<Point3<i64>, Point3<usize>>,
	added_order: Vec<Point3<i64>>,
	removed: HashSet<Point3<i64>>,
}

impl ClientCache {
	pub(crate) fn new() -> Self {
		Self {
			loaded_chunks: HashMap::new(),
			added: MultiMap::new(),
			added_order: Vec::new(),
			removed: HashSet::new(),
		}
	}

	fn apply_updates(
		&mut self,
		chunk: &mut Chunk,
		updates: &Vec<(Point3<usize>, block::LookupId)>,
	) {
		let coordinate = *chunk.coordinate();
		self.added_order.push(coordinate);
		for &(point, block_id) in updates.iter() {
			self.added.insert(coordinate, point);
			chunk.block_ids.insert(point, block_id);
		}
	}

	pub fn insert_updates(
		&mut self,
		coordinate: &Point3<i64>,
		updates: &Vec<(Point3<usize>, block::LookupId)>,
	) {
		match self.loaded_chunks.get(&coordinate).cloned() {
			Some(arc_chunk) => {
				let mut chunk = arc_chunk.write().unwrap();
				self.apply_updates(&mut chunk, &updates);
			}
			None => {
				let mut chunk = Chunk::new(*coordinate);
				self.apply_updates(&mut chunk, &updates);
				self.loaded_chunks
					.insert(*coordinate, Arc::new(RwLock::new(chunk)));
			}
		}
	}

	pub fn remove(&mut self, coordinate: &Point3<i64>) {
		let _ = self.loaded_chunks.remove(coordinate);
		self.removed.insert(*coordinate);
	}

	pub fn has_pending(&self) -> bool {
		!self.added.is_empty() || !self.removed.is_empty()
	}

	pub fn take_pending(&mut self) -> Vec<Operation> {
		let mut operations = Vec::with_capacity(self.added.len() + self.removed.len());
		for coord in self.removed.drain() {
			operations.push(Operation::Remove(coord));
		}
		for coord in self.added_order.drain(..) {
			let points = match self.added.remove(&coord) {
				Some(points) => points,
				None => continue,
			};
			let arc_chunk = match self.loaded_chunks.get(&coord) {
				Some(arc) => arc,
				None => continue,
			};
			let chunk = match arc_chunk.read() {
				Ok(chunk) => chunk,
				_ => continue,
			};
			let blocks = points
				.iter()
				.filter_map(|point| match chunk.block_ids.get(&point) {
					Some(&block_id) => Some((*point, block_id)),
					None => None,
				})
				.collect();
			operations.push(Operation::Insert(coord, blocks));
		}
		operations
	}
}
