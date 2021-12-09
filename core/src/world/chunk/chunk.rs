use engine::math::nalgebra::Point3;
use std::{
	path::PathBuf,
	sync::{Arc, RwLock, Weak},
};

pub type ArcLockChunk = Arc<RwLock<Chunk>>;
pub type WeakLockChunk = Weak<RwLock<Chunk>>;
pub struct Chunk {
	coordinate: Point3<i64>,
	path_on_disk: PathBuf,
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
			"{}x{}x{}.json",
			coordinate[0], coordinate[1], coordinate[2]
		));
		world_root
	}

	pub(super) fn load_or_generate(
		coordinate: &Point3<i64>,
		settings: &super::GeneratorSettings,
	) -> ArcLockChunk {
		let path_on_disk = Self::create_path_for(settings.root_dir.clone(), &coordinate);
		Arc::new(RwLock::new(if path_on_disk.exists() {
			Self::load(path_on_disk, &coordinate)
		} else {
			Self::generate(path_on_disk, &coordinate)
		}))
	}

	pub(super) fn generate(path_on_disk: PathBuf, coordinate: &Point3<i64>) -> Self {
		// TODO: generate
		log::debug!(target: "world", "Generating chunk {}", coordinate);
		Self {
			path_on_disk,
			coordinate: *coordinate,
		}
	}

	pub(super) fn load(path_on_disk: PathBuf, coordinate: &Point3<i64>) -> Self {
		// TODO: Load chunk from file
		log::debug!(target: "world", "Loading chunk {}", coordinate);
		Self {
			path_on_disk,
			coordinate: *coordinate,
		}
	}

	pub(super) fn save(&self) {
		let _path = &self.path_on_disk;
		log::debug!(target: "world", "Saving chunk {}", self.coordinate);
		// TODO: Save chunk to disk
	}
}
