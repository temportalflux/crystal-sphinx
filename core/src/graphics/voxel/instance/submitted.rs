use crate::{
	block,
	graphics::voxel::instance::{local::Description as LocalDescription, Instance},
};
use engine::{
	graphics::{self, buffer::Buffer, command, flags, utility::NamedObject, RenderChain},
	task::{self, ScheduledTask},
	utility::{AnyError, VoidResult},
};
use std::sync::Arc;

pub struct Description {
	pub(crate) categories: Vec<Category>,
	pub(crate) buffer: Arc<Buffer>,
}

pub struct Category {
	pub(crate) id: block::LookupId,
	pub(crate) start: usize,
	pub(crate) count: usize,
}

impl Description {
	pub fn new(render_chain: &RenderChain, instance_buffer_size: usize) -> Result<Self, AnyError> {
		let buffer = Buffer::create_gpu(
			Some(format!("RenderVoxel.InstanceBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::VERTEX_BUFFER,
			instance_buffer_size,
			None,
		)?;

		Ok(Self {
			categories: Vec::new(),
			buffer,
		})
	}

	pub fn submit(
		&mut self,
		local: &LocalDescription,
		render_chain: &RenderChain,
		pending_gpu_signals: &mut Vec<Arc<command::Semaphore>>,
	) -> VoidResult {
		self.categories.clear();

		let mut all_instances: Vec<Instance> = Vec::new();
		{
			profiling::scope!("gather-instances");
			for id in LocalDescription::ordered_ids() {
				let mut id_instances = local.get_instances(&id).clone();
				self.categories.push(Category {
					id,
					start: all_instances.len(),
					count: id_instances.len(),
				});
				all_instances.append(&mut id_instances);
			}
		}

		// If there are no blocks at all, thats ok, we dont have to write anything.
		// The metadata will prevent anything from being rendered.
		if !all_instances.is_empty() {
			graphics::TaskGpuCopy::new(
				self.buffer.wrap_name(|v| format!("Write({})", v)),
				&render_chain,
			)?
			.begin()?
			.stage(&all_instances[..])?
			.copy_stage_to_buffer(&self.buffer)
			.end()?
			.add_signal_to(pending_gpu_signals)
			.send_to(task::sender());
		}

		Ok(())
	}
}
