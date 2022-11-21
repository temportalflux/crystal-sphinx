use crate::block;
use engine::{asset, math::nalgebra::Point3};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	sync::{Arc, Mutex, Weak},
};

pub enum WorldDelta {
	ChunkInserted(Point3<i64>, Vec<Weak<BlockData>>),
	ChunkDropped(Point3<i64>, Vec<Arc<BlockData>>),
	BlockInserted(Point3<i64>, Weak<BlockData>),
	BlockDropped(Point3<i64>, Arc<BlockData>),
}

#[derive(Clone)]
pub struct Chunk {
	/// The coordinate of the chunk in the world.
	coordinate: Point3<i64>,
	blocks: HashMap<Point3<usize>, Arc<BlockData>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkSaveData {
	coordinate: Point3<i64>,
	block_ids: HashMap<Point3<usize>, block::LookupId>,
}

pub struct BlockData {
	offset: Point3<usize>,
	id: block::LookupId,
	collider_handle: Mutex<Option<crate::common::physics::backend::ColliderHandle>>,
}

impl BlockData {
	pub fn id(&self) -> &block::LookupId {
		&self.id
	}
}

impl From<(Point3<usize>, block::LookupId)> for BlockData {
	fn from((offset, id): (Point3<usize>, block::LookupId)) -> Self {
		Self {
			offset,
			id,
			collider_handle: Mutex::new(None),
		}
	}
}

impl BlockData {
	fn new(offset: Point3<usize>, id: block::LookupId) -> Self {
		Self {
			offset,
			id,
			collider_handle: Mutex::new(None),
		}
	}
}

impl From<ChunkSaveData> for Chunk {
	fn from(save_data: ChunkSaveData) -> Self {
		let blocks = save_data
			.block_ids
			.into_iter()
			.map(|(point, id)| (point, Arc::new(BlockData::from((point, id)))))
			.collect();
		Self {
			coordinate: save_data.coordinate,
			blocks,
		}
	}
}

impl Chunk {
	pub fn new(coordinate: Point3<i64>) -> Self {
		Self {
			coordinate,
			blocks: HashMap::new(),
		}
	}

	pub fn coordinate(&self) -> &Point3<i64> {
		&self.coordinate
	}

	pub fn blocks(&self) -> &HashMap<Point3<usize>, Arc<BlockData>> {
		&self.blocks
	}

	pub fn block_ids(&self) -> HashMap<Point3<usize>, block::LookupId> {
		self.blocks
			.iter()
			.map(|(point, data)| (*point, data.id))
			.collect()
	}

	pub fn to_save_data(&self) -> ChunkSaveData {
		ChunkSaveData {
			coordinate: self.coordinate,
			block_ids: self.block_ids(),
		}
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
				let data = Arc::new(BlockData::new(point, block_id));
				self.blocks.insert(point, data.clone());
			}
			None => {
				self.blocks.remove(&point);
			}
		}
	}
}
