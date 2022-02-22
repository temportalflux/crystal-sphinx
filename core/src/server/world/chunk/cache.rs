use crate::server::world::chunk::Chunk;
use engine::math::nalgebra::Point3;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock, Weak},
};

pub type ArcLock = Arc<RwLock<Cache>>;
pub type WeakLock = Weak<RwLock<Cache>>;

/// A storage bin for all the chunks which are loaded.
/// This cache stores weak references (not strong references).
///
/// It is possible (albeit unlikely) for a chunk to be present in the cache,
/// but be unloaded in a number of milliseconds because it has expired.
pub struct Cache {
	loaded_chunks: HashMap<Point3<i64>, Weak<RwLock<Chunk>>>,
}

impl Cache {
	pub fn new() -> Self {
		Self {
			loaded_chunks: HashMap::new(),
		}
	}

	pub fn insert(&mut self, coordinate: Point3<i64>, chunk: Weak<RwLock<Chunk>>) {
		let _ = self.loaded_chunks.insert(coordinate, chunk);
	}

	pub fn remove(&mut self, coordinate: &Point3<i64>) {
		let _ = self.loaded_chunks.remove(coordinate);
	}

	pub fn find(&self, coordinate: &Point3<i64>) -> Option<&Weak<RwLock<Chunk>>> {
		profiling::scope!(
			"find-server-chunk",
			&format!("<{}, {}, {}>", coordinate.x, coordinate.y, coordinate.z)
		);
		self.loaded_chunks.get(coordinate)
	}
}
