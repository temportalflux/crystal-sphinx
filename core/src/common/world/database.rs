use crate::common::world::Point;
use engine::{channels::future, math::nalgebra::Point3};
use std::{
	collections::HashMap,
	sync::{Arc, RwLock, Weak},
};

/// Holds ownership over the state of the world. Chunks can be inserted/removed as they are loaded or replicated,
/// and individual block coordinates can be updated/queried for loaded chunks.
/// When the world state changes, the update channel is broadcasted to, enabling listeners to react to chunks or blocks being changed.
pub struct Database {
	chunks: HashMap<Point3<i64>, Entry>,
	update_channel: future::Pair<Update<()>>, // TODO: Might need to use engine::broadcast instead of engine::future.
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
}

pub enum Update<T> {
	Inserted(Point3<i64>, Vec<Weak<T>>),
	Dropped(Point3<i64>, Vec<Arc<T>>),
}

impl Database {
	pub fn new() -> Self {
		let update_channel = future::unbounded();
		Self {
			chunks: HashMap::new(),
			update_channel,
		}
	}

	pub fn recv_updates(&self) -> &future::Receiver<Update<()>> {
		&self.update_channel.1
	}

	pub fn insert_chunk<T>(&mut self, coordinate: Point3<i64>, chunk: T)
	where
		T: Into<Entry>,
	{
		let previous = self.chunks.insert(coordinate, chunk.into());
		if let Some(_prev) = previous {
			// TODO: Extract the block data from the chunk (previous) and send it through the channel.
			// The chunk is about to be dropped in this frame.
			let _ = self
				.update_channel
				.0
				.try_send(Update::Dropped(coordinate, vec![]));
		}
		// TODO: Clone the block data from the chunk (chunk/new/given) and send it through the channel.
		let _ = self
			.update_channel
			.0
			.try_send(Update::Inserted(coordinate, vec![]));
	}

	pub fn remove_chunk(&mut self, coordinate: &Point3<i64>) {
		let previous = self.chunks.remove(coordinate);
		if let Some(_prev) = previous {
			// TODO: Extract the block data from the chunk and send it through the channel.
			// The chunk is about to be dropped in this frame.
			let _ = self
				.update_channel
				.0
				.try_send(Update::Dropped(*coordinate, vec![]));
		}
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
