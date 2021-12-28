use crate::graphics::voxel::model::{Model, Vertex};
use engine::asset;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

pub type ArcLockCache = Arc<RwLock<Cache>>;
pub struct Cache {
	models: HashMap<asset::Id, (Model, /*vertex offset*/ usize)>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
}

impl Cache {
	pub fn new() -> Self {
		Self {
			models: HashMap::new(),
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	pub fn arclocked(self) -> ArcLockCache {
		Arc::new(RwLock::new(self))
	}

	pub fn insert(&mut self, block_id: asset::Id, model: Model) {
		use crate::graphics::model::Model;
		let vertex_offset = self.vertices.len();
		self.vertices.append(&mut model.vertices().clone());
		self.indices.append(&mut model.indices().clone());
		self.models.insert(block_id, (model, vertex_offset));
	}

	pub fn vertex_buffer_size(&self) -> usize {
		std::mem::size_of::<Vertex>() * self.vertices.len()
	}

	pub fn index_buffer_size(&self) -> usize {
		std::mem::size_of::<u32>() * self.indices.len()
	}

	pub fn buffer_data(&self) -> (&Vec<Vertex>, &Vec<u32>) {
		(&self.vertices, &self.indices)
	}
}
