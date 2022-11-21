use crate::common::world::Point;
use engine::{channels::broadcast, math::nalgebra::Point3};
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

/// Holds ownership over the state of the world. Chunks can be inserted/removed as they are loaded or replicated,
/// and individual block coordinates can be updated/queried for loaded chunks.
/// When the world state changes, the update channel is broadcasted to, enabling listeners to react to chunks or blocks being changed.
pub struct Database {
	chunks: HashMap<Point3<i64>, Entry>,
	update_channel: broadcast::Bus<UpdateBlockId>,
}

#[derive(Clone)]
pub enum Entry {
	Server(Arc<RwLock<crate::server::world::chunk::Chunk>>),
	Client(Arc<RwLock<crate::common::world::chunk::Chunk>>),
}
impl From<Arc<RwLock<crate::server::world::chunk::Chunk>>> for Entry {
	fn from(arc: Arc<RwLock<crate::server::world::chunk::Chunk>>) -> Self {
		Self::Server(arc)
	}
}
impl From<Arc<RwLock<crate::common::world::chunk::Chunk>>> for Entry {
	fn from(arc: Arc<RwLock<crate::common::world::chunk::Chunk>>) -> Self {
		Self::Client(arc)
	}
}
impl Entry {
	pub fn unwrap_server(&self) -> &Arc<RwLock<crate::server::world::chunk::Chunk>> {
		match self {
			Self::Server(chunk) => chunk,
			Self::Client(_) => unimplemented!(),
		}
	}

	pub fn unwrap_client(&self) -> &Arc<RwLock<crate::common::world::chunk::Chunk>> {
		match self {
			Self::Client(chunk) => chunk,
			Self::Server(_) => unimplemented!(),
		}
	}

	pub fn map_chunk<R, F>(&self, f: F) -> R
	where
		F: FnOnce(&super::chunk::Chunk) -> R + 'static,
	{
		match self {
			Self::Server(chunk) => {
				let read_chunk = chunk.read().unwrap();
				f(&read_chunk.chunk)
			}
			Self::Client(chunk) => {
				let read_chunk = chunk.read().unwrap();
				f(&*read_chunk)
			}
		}
	}

	pub fn block_ids(&self) -> Vec<(Point3<usize>, crate::block::LookupId)> {
		self.map_chunk(|chunk| chunk.block_ids().into_iter().collect::<Vec<_>>())
	}
}

pub type UpdateBlockId = Update<(Point3<usize>, crate::block::LookupId)>;

#[derive(Clone)]
pub enum Update<T> {
	Inserted(Point3<i64>, Arc<Vec<T>>),
	Dropped(Point3<i64>, Arc<Vec<T>>),
}
impl<T> std::fmt::Debug for Update<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Inserted(coord, entries) => write!(
				f,
				"Inserted(<{}, {}, {}>, {} entries)",
				coord.x,
				coord.y,
				coord.z,
				entries.len()
			),
			Self::Dropped(coord, entries) => write!(
				f,
				"Dropped(<{}, {}, {}>, {} entries)",
				coord.x,
				coord.y,
				coord.z,
				entries.len()
			),
		}
	}
}

impl Database {
	pub fn new() -> Self {
		Self {
			chunks: HashMap::new(),
			update_channel: broadcast::Bus::new(100),
		}
	}

	pub fn add_recv(&mut self) -> broadcast::BusReader<UpdateBlockId> {
		self.update_channel.add_rx()
	}

	pub fn insert_chunk<T>(&mut self, coordinate: Point3<i64>, chunk: T)
	where
		Entry: From<T>,
	{
		let entry = Entry::from(chunk);
		let insert_updates = entry.block_ids();
		let previous = self.chunks.insert(coordinate, entry);
		if let Some(prev) = previous {
			// Extract the block data from the chunk (previous) and send it through the channel.
			// The chunk is about to be dropped in this frame.
			self.update_channel
				.broadcast(Update::Dropped(coordinate, Arc::new(prev.block_ids())));
		}
		// Clone the block data from the chunk (chunk/new/given) and send it through the channel.
		self.update_channel
			.broadcast(Update::Inserted(coordinate, Arc::new(insert_updates)));
	}

	pub fn remove_chunk(&mut self, coordinate: &Point3<i64>) -> Option<Entry> {
		let previous = self.chunks.remove(coordinate);
		if let Some(prev) = &previous {
			// Extract the block data from the chunk and send it through the channel.
			// The chunk is about to be dropped in this frame.
			self.update_channel
				.broadcast(Update::Dropped(*coordinate, Arc::new(prev.block_ids())));
		}
		previous
	}

	pub fn find_chunk(&self, coordinate: &Point3<i64>) -> Option<&Entry> {
		profiling::scope!(
			"find-server-chunk",
			&format!("<{}, {}, {}>", coordinate.x, coordinate.y, coordinate.z)
		);
		self.chunks.get(coordinate)
	}

	pub fn insert_block(&self, _point: Point<i8>) -> Option<()> {
		None
	}

	pub fn remove_block(&self, _point: Point<i8>) -> Option<()> {
		None
	}
}
