use crate::graphics::voxel::{
	atlas::Atlas,
	model::{Model, Vertex},
};
use std::sync::{Arc, RwLock};

pub type ArcLockCache = Arc<RwLock<Cache>>;
pub struct Cache {
	models: Vec<(Model, /*vertex offset*/ usize)>,
	atlases: Vec<Arc<Atlas>>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
}

impl Cache {
	pub fn new() -> Self {
		Self {
			models: Vec::new(),
			atlases: Vec::new(),
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	pub fn arclocked(self) -> ArcLockCache {
		Arc::new(RwLock::new(self))
	}

	pub(crate) fn add_atlas(&mut self, atlas: Arc<Atlas>) {
		self.atlases.push(atlas);
	}

	pub fn insert(&mut self, mut model: Model) {
		use crate::graphics::model::Model;
		model = model.build_data();
		let vertex_offset = self.vertices.len();
		self.vertices.append(&mut model.vertices().clone());
		self.indices.append(&mut model.indices().clone());
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
