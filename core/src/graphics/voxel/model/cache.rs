use crate::{
	block,
	graphics::voxel::model::{Model, Vertex},
};
use engine::{
	graphics::{
		buffer, command::Semaphore, descriptor, flags, utility::NamedObject, DescriptorCache,
		RenderChain, TaskGpuCopy,
	},
	task::{self, ScheduledTask},
	utility::Result,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct CacheBuilder {
	models: HashMap<
		block::LookupId,
		(
			Model,
			/*index start*/ usize,
			/*vertex offset*/ usize,
		),
	>,
	atlas_descriptor_cache: Option<DescriptorCache<(usize, usize)>>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
}

impl CacheBuilder {
	pub fn insert(&mut self, block_id: block::LookupId, model: Model) {
		use crate::graphics::model::Model;
		let index_start = self.indices.len();
		let vertex_offset = self.vertices.len();
		self.vertices.append(&mut model.vertices().clone());
		self.indices.append(&mut model.indices().clone());
		self.models
			.insert(block_id, (model, index_start, vertex_offset));
	}

	pub fn set_atlas_descriptor_cache(&mut self, cache: DescriptorCache<(usize, usize)>) {
		self.atlas_descriptor_cache = Some(cache);
	}

	pub fn build(self, render_chain: &RenderChain) -> Result<(Cache, Vec<Arc<Semaphore>>)> {
		Cache::create(self, render_chain)
	}
}

pub struct Cache {
	models: HashMap<
		block::LookupId,
		(
			Model,
			/*index start*/ usize,
			/*vertex offset*/ usize,
		),
	>,
	atlas_descriptor_cache: DescriptorCache<(usize, usize)>,
	pub(crate) vertex_buffer: Arc<buffer::Buffer>,
	pub(crate) index_buffer: Arc<buffer::Buffer>,
}

impl Cache {
	pub fn builder() -> CacheBuilder {
		CacheBuilder::default()
	}

	fn create(
		builder: CacheBuilder,
		render_chain: &RenderChain,
	) -> Result<(Self, Vec<Arc<Semaphore>>)> {
		let vbuff_size = std::mem::size_of::<Vertex>() * builder.vertices.len();
		let ibuff_size = std::mem::size_of::<u32>() * builder.indices.len();
		let mut pending_gpu_signals = Vec::with_capacity(/*buffer writes*/ 2);

		let (vertex_buffer, index_buffer) = {
			let vertex_buffer = buffer::Buffer::create_gpu(
				Some("RenderVoxel.VertexBuffer".to_owned()),
				&render_chain.allocator(),
				flags::BufferUsage::VERTEX_BUFFER,
				vbuff_size,
				None,
			)?;

			TaskGpuCopy::new(
				vertex_buffer.wrap_name(|v| format!("Write({})", v)),
				&render_chain,
			)?
			.begin()?
			.stage(&builder.vertices[..])?
			.copy_stage_to_buffer(&vertex_buffer)
			.end()?
			.add_signal_to(&mut pending_gpu_signals)
			.send_to(task::sender());

			let index_buffer = buffer::Buffer::create_gpu(
				Some("RenderVoxel.IndexBuffer".to_owned()),
				&render_chain.allocator(),
				flags::BufferUsage::INDEX_BUFFER,
				ibuff_size,
				Some(flags::IndexType::UINT32),
			)?;

			TaskGpuCopy::new(
				index_buffer.wrap_name(|v| format!("Write({})", v)),
				&render_chain,
			)?
			.begin()?
			.stage(&builder.indices[..])?
			.copy_stage_to_buffer(&index_buffer)
			.end()?
			.add_signal_to(&mut pending_gpu_signals)
			.send_to(task::sender());

			(vertex_buffer, index_buffer)
		};

		Ok((
			Self {
				models: builder.models,
				atlas_descriptor_cache: builder.atlas_descriptor_cache.unwrap(),
				vertex_buffer,
				index_buffer,
			},
			pending_gpu_signals,
		))
	}

	pub fn descriptor_layout(&self) -> &Arc<descriptor::layout::SetLayout> {
		self.atlas_descriptor_cache.layout()
	}

	pub fn get(
		&self,
		id: &block::LookupId,
	) -> Option<&(
		Model,
		/*index start*/ usize,
		/*vertex offset*/ usize,
	)> {
		self.models.get(&id)
	}
}
