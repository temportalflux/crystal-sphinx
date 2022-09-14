use super::{Model, Vertex};
use anyhow::Result;
use engine::{
	asset,
	channels::mpsc::Sender,
	graphics::{
		buffer, command::Semaphore, flags, utility::NamedObject, GpuOpContext, GpuOperationBuilder,
	},
};
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct CacheBuilder {
	models: HashMap<
		asset::Id,
		(
			Model,
			/*index start*/ usize,
			/*vertex offset*/ usize,
		),
	>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
}

impl CacheBuilder {
	pub fn with_capacity(model_count: usize) -> Self {
		Self {
			models: HashMap::with_capacity(model_count),
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	pub fn insert(&mut self, id: asset::Id, model: Model) {
		use crate::graphics::model::Model;
		let index_start = self.indices.len();
		let vertex_offset = self.vertices.len();
		self.vertices.append(&mut model.vertices().clone());
		self.indices.append(&mut model.indices().clone());
		self.models.insert(id, (model, index_start, vertex_offset));
	}

	pub fn build(
		self,
		context: &impl GpuOpContext,
		name: &str,
		signal_sender: &Sender<Arc<Semaphore>>,
	) -> Result<Cache> {
		Cache::create(self, context, name, signal_sender)
	}
}

pub struct Cache {
	models: HashMap<
		asset::Id,
		(
			Model,
			/*index start*/ usize,
			/*vertex offset*/ usize,
		),
	>,
	pub(crate) vertex_buffer: Arc<buffer::Buffer>,
	pub(crate) index_buffer: Arc<buffer::Buffer>,
}

impl Cache {
	pub fn builder() -> CacheBuilder {
		CacheBuilder::default()
	}

	fn create(
		builder: CacheBuilder,
		context: &impl GpuOpContext,
		name: &str,
		signal_sender: &Sender<Arc<Semaphore>>,
	) -> Result<Self> {
		let vbuff_size = std::mem::size_of::<Vertex>() * builder.vertices.len();
		let ibuff_size = std::mem::size_of::<u32>() * builder.indices.len();

		let (vertex_buffer, index_buffer) = {
			let vertex_buffer = buffer::Buffer::create_gpu(
				Some(format!("{name}.VertexBuffer")),
				&context.object_allocator()?,
				flags::BufferUsage::VERTEX_BUFFER,
				vbuff_size,
				None,
			)?;

			GpuOperationBuilder::new(
				vertex_buffer.wrap_name(|v| format!("Write({})", v)),
				context,
			)?
			.begin()?
			.stage(&builder.vertices[..])?
			.copy_stage_to_buffer(&vertex_buffer)
			.send_signal_to(signal_sender)?
			.end()?;

			let index_buffer = buffer::Buffer::create_gpu(
				Some(format!("{name}.IndexBuffer")),
				&context.object_allocator()?,
				flags::BufferUsage::INDEX_BUFFER,
				ibuff_size,
				Some(flags::IndexType::UINT32),
			)?;

			GpuOperationBuilder::new(index_buffer.wrap_name(|v| format!("Write({})", v)), context)?
				.begin()?
				.stage(&builder.indices[..])?
				.copy_stage_to_buffer(&index_buffer)
				.send_signal_to(signal_sender)?
				.end()?;

			(vertex_buffer, index_buffer)
		};

		Ok(Self {
			models: builder.models,
			vertex_buffer,
			index_buffer,
		})
	}

	pub fn get(
		&self,
		id: &asset::Id,
	) -> Option<&(
		Model,
		/*index start*/ usize,
		/*vertex offset*/ usize,
	)> {
		self.models.get(&id)
	}
}
