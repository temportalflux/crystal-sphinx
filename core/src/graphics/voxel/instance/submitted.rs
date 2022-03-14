use crate::graphics::voxel::instance::{category::Category, local::IntegratedBuffer, Instance};
use anyhow::Result;
use engine::channels::mpsc::Sender;
use engine::graphics::{
	alloc, buffer::Buffer, command, flags, utility::NamedObject, GpuOpContext, GpuOperationBuilder,
};
use std::sync::Arc;

pub struct Description {
	pub(crate) categories: Vec<Category>,
	pub(crate) buffer: Arc<Buffer>,
}

impl Description {
	pub fn new(allocator: &Arc<alloc::Allocator>, instance_buffer_size: usize) -> Result<Self> {
		let buffer = Buffer::create_gpu(
			Some(format!("RenderVoxel.InstanceBuffer")),
			allocator,
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
		context: &impl GpuOpContext,
		signal_sender: &Sender<Arc<command::Semaphore>>,
	) -> Result<()> {
		self.categories.clear();

		let mut ranges = Vec::with_capacity(changed_ranges.len());
		let instance_size = std::mem::size_of::<Instance>();

		let mut task = {
			profiling::scope!("prepare-task");
			let mut task = GpuOperationBuilder::new(
				self.buffer.wrap_name(|v| format!("Write({})", v)),
				context,
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
				.send_signal_to(signal_sender)?
				.end()?;
		}

		self.categories = local.get_categories().clone();

		Ok(())
	}
}
