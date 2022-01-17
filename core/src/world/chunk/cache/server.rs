use crate::world::chunk::ServerChunk;
use engine::math::nalgebra::Point3;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock, Weak},
};

pub type ArcLockServerCache = Arc<RwLock<ServerCache>>;
pub type WeakLockServerCache = Weak<RwLock<ServerCache>>;

/// A storage bin for all the chunks which are loaded.
/// This cache stores weak references (not strong references).
///
/// It is possible (albeit unlikely) for a chunk to be present in the cache,
/// but be unloaded in a number of milliseconds because it has expired.
pub struct ServerCache {
	loaded_chunks: HashMap<Point3<i64>, Weak<RwLock<ServerChunk>>>,
}

impl ServerCache {
	pub fn new() -> Self {
		Self {
			loaded_chunks: HashMap::new(),
		}
	}

	pub fn insert(&mut self, coordinate: Point3<i64>, chunk: Weak<RwLock<ServerChunk>>) {
		let _ = self.loaded_chunks.insert(coordinate, chunk);
	}

	pub fn remove(&mut self, coordinate: &Point3<i64>) {
		let _ = self.loaded_chunks.remove(coordinate);
	}

	pub fn find(&self, coordinate: &Point3<i64>) -> Option<&Weak<RwLock<ServerChunk>>> {
		self.loaded_chunks.get(coordinate)
	}
}
