use crate::{
	common::world::{chunk::Chunk as CommonChunk, generator},
	server::world::chunk::Level,
};
use engine::math::nalgebra::Point3;
use std::{
	path::PathBuf,
	sync::{Arc, RwLock},
};

pub type ArcLock = Arc<RwLock<Chunk>>;

/// A 16x16x16 chunk in the world.
///
/// Data is saved to disk at `<world root>/chunks/x.y.z.kdl`.
pub struct Chunk {
	pub chunk: CommonChunk,
	/// The path to the chunk on disk.
	/// Not saved to file.
	path_on_disk: PathBuf,
	/// The current ticking level of the chunk.
	/// Not saved to file.
	pub(crate) level: Level,
}

impl Chunk {
	fn create_path_for(mut world_root: PathBuf, coordinate: &Point3<i64>) -> PathBuf {
		world_root.push("chunks");
		world_root.push(format!(
			"{}.{}.{}.kdl",
			coordinate[0], coordinate[1], coordinate[2]
		));
		world_root
	}

	pub(super) fn load_or_generate(
		coordinate: &Point3<i64>,
		level: Level,
		root_dir: PathBuf,
	) -> Arc<RwLock<Self>> {
		let path_on_disk = Self::create_path_for(root_dir, &coordinate);
		Arc::new(RwLock::new(if path_on_disk.exists() {
			Self::load(path_on_disk, &coordinate, level)
		} else {
			Self::generate(path_on_disk, &coordinate, level)
		}))
	}

	pub(super) fn generate(path_on_disk: PathBuf, coordinate: &Point3<i64>, level: Level) -> Self {
		profiling::scope!("generate-chunk", path_on_disk.to_str().unwrap_or(""));
		//log::debug!(target: "world", "Generating chunk {}", coordinate);

		let generator = generator::Flat::classic();
		let chunk = generator.generate_chunk(*coordinate);

		Self {
			path_on_disk,
			chunk,
			level,
		}
	}

	pub(super) fn load(path_on_disk: PathBuf, coordinate: &Point3<i64>, level: Level) -> Self {
		profiling::scope!("load-chunk", path_on_disk.to_str().unwrap_or(""));
		// TODO: Load chunk from file
		//log::debug!(target: "world", "Loading chunk {}", coordinate);
		Self {
			path_on_disk,
			chunk: CommonChunk::new(*coordinate),
			level,
		}
	}

	pub(super) fn save(&self) {
		profiling::scope!("save-chunk", self.path_on_disk.to_str().unwrap_or(""));
		let _path = &self.path_on_disk;
		//log::debug!(target: "world", "Saving chunk {}", self.coordinate);
		// TODO: Save chunk to disk
	}
}
