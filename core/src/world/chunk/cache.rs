use super::WeakLockChunk;
use crate::world::Settings;
use engine::math::nalgebra::Point3;
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, RwLock},
};

#[derive(Clone)]
pub struct GeneratorSettings {
	pub(super) root_dir: PathBuf,
	pub(super) _seed: String,
}

/// Alias for Arc<RwLock<[`Cache`](Cache)>>.
pub type ArcLockCache = Arc<RwLock<Cache>>;

/// A storage bin for all the chunks which are loaded.
/// This cache stores weak references (not strong references).
///
/// It is possible (albeit unlikely) for a chunk to be present in the cache,
/// but be unloaded in a number of milliseconds because it has expired.
pub struct Cache {
	loaded_chunks: HashMap<Point3<i64>, WeakLockChunk>,
	pub(crate) world_gen_settings: GeneratorSettings,
}
impl Cache {
	pub(crate) fn new(settings: &Settings) -> Self {
		Self {
			loaded_chunks: HashMap::new(),
			world_gen_settings: GeneratorSettings {
				root_dir: settings.root_path().to_owned(),
				_seed: settings.seed().to_owned(),
			},
		}
	}

	pub fn find(&self, coordinate: &Point3<i64>) -> Option<&WeakLockChunk> {
		self.loaded_chunks.get(coordinate)
	}

	pub(crate) fn insert(&mut self, coordinate: &Point3<i64>, chunk: WeakLockChunk) {
		let old_value = self.loaded_chunks.insert(*coordinate, chunk);
		assert!(old_value.is_none());
	}

	pub(crate) fn remove(&mut self, coordinate: &Point3<i64>) {
		let old_value = self.loaded_chunks.remove(coordinate);
		assert!(old_value.is_some());
	}
}
