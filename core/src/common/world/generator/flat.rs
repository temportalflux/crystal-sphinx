use crate::{
	block,
	common::world::chunk::{self, Chunk},
};
use engine::{asset, math::nalgebra::Point3};
use std::collections::HashMap;

#[derive(Default)]
pub struct Flat {
	layers: HashMap</*chunk-y*/ i64, HashMap</*block-y*/ usize, block::LookupId>>,
	glass_id: block::LookupId,
	debug_id: block::LookupId,
}

impl Flat {
	pub fn classic() -> Self {
		let mut cfg = Self::default();

		cfg.insert((0, 0), &asset::Id::new("vanilla", "blocks/bedrock"));

		let stone = asset::Id::new("vanilla", "blocks/stone");
		cfg.insert((0, 1), &stone);
		cfg.insert((0, 2), &stone);
		cfg.insert((0, 3), &stone);

		let dirt = asset::Id::new("vanilla", "blocks/dirt");
		cfg.insert((0, 4), &dirt);
		cfg.insert((0, 5), &dirt);

		cfg.insert((0, 6), &asset::Id::new("vanilla", "blocks/grass/default"));

		cfg.glass_id = Self::lookup(&asset::Id::new("vanilla", "blocks/glass/clear")).unwrap();
		cfg.debug_id = Self::lookup(&asset::Id::new("crystal-sphinx", "blocks/debug")).unwrap();

		cfg
	}

	fn lookup(id: &asset::Id) -> Option<block::LookupId> {
		block::Lookup::lookup_value(&id)
	}

	pub fn insert(&mut self, layer: (i64, usize), id: &asset::Id) {
		let id = match Self::lookup(&id) {
			Some(id) => id,
			None => return,
		};
		if !self.layers.contains_key(&layer.0) {
			self.layers.insert(layer.0, HashMap::new());
		}
		let chunk_layer = self.layers.get_mut(&layer.0).unwrap();
		chunk_layer.insert(layer.1, id);
	}

	pub fn generate_chunk(&self, coordinate: Point3<i64>) -> Chunk {
		use rand::prelude::*;
		let mut rng = rand::thread_rng();
		let mut chunk = Chunk::new(coordinate);
		
		if let Some(layers) = self.layers.get(&coordinate.y) {
			for y in 0..chunk::SIZE_I.y {
				if let Some(&block_id) = layers.get(&y) {
					for x in 1..chunk::SIZE_I.x - 1 {
						for z in 1..chunk::SIZE_I.z - 1 {
							if y > 0 {
								let chance = rng.gen::<usize>() % 100;
								if chance < 15 {
									chunk.set_block_id(Point3::new(x, y, z), Some(self.glass_id));
									continue;
								}
							}

							chunk.set_block_id(Point3::new(x, y, z), Some(block_id));
						}
					}
				}
			}
		}
		
		if coordinate == Point3::origin() {
			chunk.set_block_id(Point3::new(8, 10, 8), Some(self.debug_id));
		}

		chunk
	}
}
