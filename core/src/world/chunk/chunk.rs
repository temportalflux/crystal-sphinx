use crate::{
	block,
	world::{chunk::Level, generator},
};
use engine::{asset, math::nalgebra::Point3};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, RwLock},
};

/// A 16x16x16 chunk in the world.
///
/// Data is saved to disk at `<world root>/chunks/x.y.z.kdl`.
pub struct ServerChunk {
	pub chunk: Chunk,
	/// The path to the chunk on disk.
	/// Not saved to file.
	path_on_disk: PathBuf,
	/// The current ticking level of the chunk.
	/// Not saved to file.
	pub(crate) level: Level,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Chunk {
	/// The coordinate of the chunk in the world.
	pub(crate) coordinate: Point3<i64>,
	pub(crate) block_ids: HashMap<Point3<usize>, block::LookupId>,
}

impl Chunk {
	pub fn new(coordinate: Point3<i64>) -> Self {
		Self {
			coordinate,
			block_ids: HashMap::new(),
		}
	}

	pub fn coordinate(&self) -> &Point3<i64> {
		&self.coordinate
	}

	pub fn block_ids(&self) -> &HashMap<Point3<usize>, block::LookupId> {
		&self.block_ids
	}

	pub fn set_block(&mut self, point: Point3<usize>, id: Option<&asset::Id>) {
		let id = match id {
			Some(asset_id) => match block::Lookup::lookup_value(&asset_id) {
				Some(id) => Some(id),
				None => return,
			},
			None => None,
		};
		self.set_block_id(point, id);
	}

	pub fn set_block_id(&mut self, point: Point3<usize>, id: Option<block::LookupId>) {
		match id {
			Some(block_id) => {
				self.block_ids.insert(point, block_id);
			}
			None => {
				self.block_ids.remove(&point);
			}
		}
	}
}

impl ServerChunk {
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
	) -> Arc<RwLock<ServerChunk>> {
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
			chunk: Chunk::new(*coordinate),
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
