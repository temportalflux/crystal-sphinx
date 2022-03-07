use crate::{
	client::world::chunk::{Operation, OperationReceiver as ChunkOperationReceiver},
	common::world::chunk,
	graphics::voxel::{
		instance::{local, submitted, Instance},
		model,
	},
};
use anyhow::Result;
use engine::{
	graphics::{alloc, command, Chain, RenderChain},
	utility::{self},
};
use std::sync::{Arc, Mutex, Weak};

static LOG: &'static str = "voxel-instance-buffer";

/// Controls the instance buffer data for rendering voxels.
/// Keeps track of what chunks and blocks are old and updates the instances accordingly.
pub struct Buffer {
	local_integrated_buffer: Arc<Mutex<local::IntegratedBuffer>>,
	submitted_description: submitted::Description,
	_thread_handle: Arc<()>,
}

impl Buffer {
	pub fn new(
		allocator: &Arc<alloc::Allocator>,
		model_cache: Weak<model::Cache>,
		chunk_receiver: ChunkOperationReceiver,
	) -> Result<Self> {
		// TODO: Get this value from settings
		let render_radius = 6;
		// square diameter of the cube surrounding the player
		let render_diameter = render_radius * 2 + 1;
		let rendered_chunk_count = render_diameter * render_diameter * render_diameter;

		/*
		Chunk volume = 16^3 blocks
		Max blocks per chunk = (16^3) = ((2^4)^3) = 2^12
		siceof(Instance) = 96 bytes -> rounded up to nearest pow2 = 128 bytes = 2^7
		2^12 * sizeof(Instance) = 2^12 * 2^7 = 2^17 = 131,072 bytes for 1 chunk

		(2x+1)^3 = number of chunks where x is the render radius/distance
		so total bytes = (2x+1)^3 * 2^17
		if x = 5, total bytes = 11^3 * 2^17 = 174,456,832 ~= 175 MB
		this is the worst cast scenario where every block in a chunk needs to render.
		It is far more likely that approximately half of all blocks in a given chunk
		will actually have faces to render, in which case the instances used per chunk is ~= (16^3)/2
		which reduces the bytes-per-chunk to 2^16=65,536, and the total bytes for x=5 to 87,228,416 ~= 88 MB.
		*/
		let chunk_volume = chunk::SIZE_I.x * chunk::SIZE_I.y * chunk::SIZE_I.z;
		let max_rendered_instances = rendered_chunk_count * chunk_volume;
		let instance_buffer_size = max_rendered_instances * std::mem::size_of::<Instance>();

		log::info!(
			target: LOG,
			"Initializing with chunk_radius={} total_chunk_count={} buffer_size={}(bytes)",
			render_radius,
			rendered_chunk_count,
			instance_buffer_size
		);

		let local_integrated_buffer = Arc::new(Mutex::new(local::IntegratedBuffer::new(
			max_rendered_instances,
			model_cache.clone(),
		)));
		let submitted_description = submitted::Description::new(allocator, instance_buffer_size)?;

		let _thread_handle =
			Self::start_thread(chunk_receiver, Arc::downgrade(&local_integrated_buffer));

		Ok(Self {
			_thread_handle,
			local_integrated_buffer,
			submitted_description,
		})
	}

	fn start_thread(
		chunk_receiver: ChunkOperationReceiver,
		description: Weak<Mutex<local::IntegratedBuffer>>,
	) -> Arc<()> {
		let handle = Arc::new(());
		let weak_handle = Arc::downgrade(&handle);
		utility::spawn_thread(LOG, move || -> Result<()> {
			use std::thread::sleep;
			use std::time::Duration;
			static LOG: &'static str = "_";
			log::info!(target: LOG, "Starting thread");
			while weak_handle.strong_count() > 0 {
				let unable_to_lock_delay_ms = 1;
				let no_chunks_to_proccess_delay_ms = 1000;
				let operation_batch_size = 10;
				let delay_between_batches = 10;

				let arc_description = match description.upgrade() {
					Some(arc) => arc,
					None => {
						sleep(Duration::from_millis(unable_to_lock_delay_ms));
						continue;
					}
				};

				let delay_ms;
				if !chunk_receiver.is_empty() {
					profiling::scope!("process");
					if let Ok(mut description) = arc_description.try_lock() {
						use anyhow::Context;
						delay_ms = delay_between_batches;
						let mut operation_count = 0;
						while let Ok(operation) = chunk_receiver.try_recv() {
							let res = match operation {
								Operation::Remove(coord) => {
									let res = description.remove_chunk(&coord);
									res.with_context(|| {
										format!(
											"remove chunk <{}, {}, {}>",
											coord.x, coord.y, coord.z
										)
									})
								}
								Operation::Insert(coord, updates) => {
									let res = description.insert_chunk(coord, updates);
									res.with_context(|| {
										format!(
											"insert chunk <{}, {}, {}>",
											coord.x, coord.y, coord.z
										)
									})
								}
							};
							if let Err(err) = res {
								log::error!(target: "thread", "{:?}", err);
							}
							operation_count += 1;
							if operation_count >= operation_batch_size {
								break;
							}
						}
					} else {
						delay_ms = unable_to_lock_delay_ms;
					}
				} else {
					delay_ms = no_chunks_to_proccess_delay_ms;
				}
				sleep(Duration::from_millis(delay_ms));
			}
			log::info!(target: LOG, "Ending thread");
			Ok(())
		});
		handle
	}

	pub fn submitted(&self) -> &submitted::Description {
		&self.submitted_description
	}

	pub fn submit_pending_changes(&mut self, chain: &Chain) -> Result<bool> {
		profiling::scope!("update_voxel_instances");
		let mut was_changed = false;
		if let Ok(mut local_description) = self.local_integrated_buffer.try_lock() {
			if let Some((changed_ranges, total_count)) = local_description.take_changed_ranges() {
				was_changed = true;
				self.submitted_description.submit(
					changed_ranges,
					total_count,
					&local_description,
					chain,
					chain.signal_sender(),
				)?;
			}
		}
		Ok(was_changed)
	}
}
