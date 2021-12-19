use super::Level;
use engine::math::nalgebra::Point3;
use std::{
	path::PathBuf,
	sync::{Arc, RwLock, Weak},
};

/// Alias for Arc<RwLock<[`Chunk`](Chunk)>>.
pub type ArcLockChunk = Arc<RwLock<Chunk>>;
/// Alias for Weak<RwLock<[`Chunk`](Chunk)>>.
pub type WeakLockChunk = Weak<RwLock<Chunk>>;

/// A 16x16x16 chunk in the world.
///
/// Data is saved to disk at `<world root>/chunks/x.y.z.kdl`.
pub struct Chunk {
	/// The coordinate of the chunk in the world.
	/// Not saved to file.
	coordinate: Point3<i64>,
	/// The path to the chunk on disk.
	/// Not saved to file.
	path_on_disk: PathBuf,
	/// The current ticking level of the chunk.
	/// Not saved to file.
	pub(crate) level: Level,
}

impl Chunk {
	pub fn coordinate(&self) -> &Point3<i64> {
		&self.coordinate
	}
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
		settings: &super::GeneratorSettings,
	) -> ArcLockChunk {
		let path_on_disk = Self::create_path_for(settings.root_dir.clone(), &coordinate);
		Arc::new(RwLock::new(if path_on_disk.exists() {
			Self::load(path_on_disk, &coordinate, level)
		} else {
			Self::generate(path_on_disk, &coordinate, level)
		}))
	}

	pub(super) fn generate(path_on_disk: PathBuf, coordinate: &Point3<i64>, level: Level) -> Self {
		profiling::scope!("generate-chunk", path_on_disk.to_str().unwrap_or(""));
		// TODO: generate
		//log::debug!(target: "world", "Generating chunk {}", coordinate);
		Self {
			path_on_disk,
			coordinate: *coordinate,
			level,
		}
	}

	pub(super) fn load(path_on_disk: PathBuf, coordinate: &Point3<i64>, level: Level) -> Self {
		profiling::scope!("load-chunk", path_on_disk.to_str().unwrap_or(""));
		// TODO: Load chunk from file
		//log::debug!(target: "world", "Loading chunk {}", coordinate);
		Self {
			path_on_disk,
			coordinate: *coordinate,
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
