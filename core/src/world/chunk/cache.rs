use super::{Chunk, ServerChunk};
use engine::math::nalgebra::Point3;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock, Weak},
};

/// A storage bin for all the chunks which are loaded.
/// This cache stores weak references (not strong references).
///
/// It is possible (albeit unlikely) for a chunk to be present in the cache,
/// but be unloaded in a number of milliseconds because it has expired.
pub type ServerCache = Cache<Weak<RwLock<ServerChunk>>>;
pub type ArcLockServerCache = Arc<RwLock<ServerCache>>;
pub type WeakLockServerCache = Weak<RwLock<ServerCache>>;
/// A storage bin for all the chunks which are relevant to the client.
/// Stores strong references until replication packets remove chunks.
pub type ClientCache = Cache<Arc<RwLock<Chunk>>>;
pub type ArcLockClientCache = Arc<RwLock<ClientCache>>;

pub struct Cache<TArcLockChunk> {
	pending: Vec<Point3<i64>>,
	removed: Vec<Point3<i64>>,
	loaded_chunks: HashMap<Point3<i64>, TArcLockChunk>,
}
impl<TArcLockChunk> Cache<TArcLockChunk> {
	pub(crate) fn new() -> Self {
		Self {
			pending: Vec::new(),
			removed: Vec::new(),
			loaded_chunks: HashMap::new(),
		}
	}

	pub fn find(&self, coordinate: &Point3<i64>) -> Option<&TArcLockChunk> {
		self.loaded_chunks.get(coordinate)
	}

	pub fn count(&self) -> usize {
		self.loaded_chunks.len()
	}

	pub(crate) fn has_pending(&self) -> bool {
		!self.pending.is_empty() || !self.removed.is_empty()
	}

	pub(crate) fn take_pending(&mut self) -> (Vec<TArcLockChunk>, Vec<Point3<i64>>)
	where
		TArcLockChunk: Clone,
	{
		let pending = self.pending.drain(..).collect::<Vec<_>>();
		let pending = pending
			.into_iter()
			.filter_map(|coord| self.find(&coord))
			.cloned()
			.collect();
		let removed = self.removed.drain(..).collect();
		(pending, removed)
	}

	pub(crate) fn insert(&mut self, coordinate: &Point3<i64>, chunk: TArcLockChunk) {
		let _ = self.loaded_chunks.insert(*coordinate, chunk);
		self.pending.push(coordinate.clone());
	}

	pub(crate) fn remove(&mut self, coordinate: &Point3<i64>) {
		let _ = self.loaded_chunks.remove(coordinate);
		self.removed.push(coordinate.clone());
	}
}
