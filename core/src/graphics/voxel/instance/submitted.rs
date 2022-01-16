use crate::graphics::voxel::instance::{category::Category, local::IntegratedBuffer, Instance};
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
		changed_ranges: Vec<std::ops::Range<usize>>,
		total_count: usize,
		local: &IntegratedBuffer,
		render_chain: &RenderChain,
		pending_gpu_signals: &mut Vec<Arc<command::Semaphore>>,
	) -> VoidResult {
		self.categories.clear();

		let mut ranges = Vec::with_capacity(changed_ranges.len());
		let instance_size = std::mem::size_of::<Instance>();

		let mut task = {
			profiling::scope!("prepare-task");
			let mut task = graphics::TaskGpuCopy::new(
				self.buffer.wrap_name(|v| format!("Write({})", v)),
				&render_chain,
			)?
			.begin()?;
			task.stage_start(total_count * instance_size)?;
			task
		};

		{
			profiling::scope!("gather-instances");
			let mut staging_memory = task.staging_memory()?;
			let mut staging_offset;
			for range in changed_ranges.into_iter() {
				let instance_offset = range.start;
				let instance_count = range.end - range.start;
				staging_offset = staging_memory.amount_written();
				staging_memory.write_slice(&local.instances()[range])?;
				ranges.push(command::CopyBufferRange {
					start_in_src: staging_offset,
					start_in_dst: instance_offset * instance_size,
					size: instance_count * instance_size,
				});
			}
		}

		{
			profiling::scope!("run-task");
			task.copy_stage_to_buffer_ranges(&self.buffer, ranges)
				.end()?
				.add_signal_to(pending_gpu_signals)
				.send_to(task::sender());
		}

		self.categories = local.get_categories().clone();

		Ok(())
	}
}