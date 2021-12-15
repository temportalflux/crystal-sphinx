use super::{Model, Vertex};
use std::sync::{Arc, RwLock};

pub type ArcLockModelCache = Arc<RwLock<ModelCache>>;
pub struct ModelCache {
	models: Vec<(Model, /*vertex offset*/ usize)>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
}

impl ModelCache {
	pub fn new() -> Self {
		Self {
			models: Vec::new(),
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	pub fn arclocked(self) -> ArcLockModelCache {
		Arc::new(RwLock::new(self))
	}

	pub fn insert(&mut self, mut model: Model) {
		use crate::graphics::model::Model;
		model = model.build_data();
		let vertex_offset = self.vertices.len();
		self.vertices.append(&mut model.vertices().to_vec());
		self.indices.append(&mut model.indices().to_vec());
		self.models.push((model, vertex_offset));
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
