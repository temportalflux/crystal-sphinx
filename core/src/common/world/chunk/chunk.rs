use crate::block;
use engine::{asset, math::nalgebra::Point3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
