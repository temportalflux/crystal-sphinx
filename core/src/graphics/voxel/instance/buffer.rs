use crate::{
	block,
	graphics::voxel::instance::{self, Instance},
	world::chunk::{self, ArcLockClientCache},
};
use engine::{
	graphics::{self, buffer, command, flags, utility::NamedObject, RenderChain},
	math::nalgebra::{Translation3, Vector3},
	task::{self, ScheduledTask},
	utility::AnyError,
};
use enumset::EnumSet;
use std::{sync::Arc, collections::HashMap};

/// Controls the instance buffer data for rendering voxels.
/// Keeps track of what chunks and blocks are old and updates the instances accordingly.
pub struct Buffer {
	chunk_cache: ArcLockClientCache,
	buffer: Arc<buffer::Buffer>,
	instances: Vec<Instance>,
	// Temporary list mapping of block id to the index values of instances for each type
	ids: HashMap<block::LookupId, Vec<usize>>,
}

impl Buffer {
	pub fn new(
		render_chain: &RenderChain,
		chunk_cache: ArcLockClientCache,
	) -> Result<Self, AnyError> {
		let render_radius = 5; // TODO: Get this value from settings
					   // square diameter of the cube surrounding the player
		let render_diameter = render_radius * 2 + 1;
		let rendered_chunk_count = render_diameter * render_diameter * render_diameter;

		/*
		Chunk volume = 16^3 blocks
		Max number of blocks in a chunk that can show all faces at once = volume / 2
		Max blocks per chunk = (16^3)/2 = ((2^4)^3)/2 = 2^11
		siceof(Instance) = 96 bytes -> rounded up to nearest pow2 = 128 bytes = 2^7
		2^11 * sizeof(Instance) = 2^11 * 2^7 = 2^16 = 65,536 bytes
		*/
		let chunk_volume = chunk::SIZE_I.x * chunk::SIZE_I.y * chunk::SIZE_I.z;
		let max_rendered_per_chunk = chunk_volume / 2;
		let max_rendered_instances = rendered_chunk_count * max_rendered_per_chunk;
		let instance_buffer_size = max_rendered_instances * std::mem::size_of::<Instance>();

		log::debug!("Initializing voxel instance buffer: chunk_radius={} total_chunk_count={}, buffer_size={}(bytes)", render_radius, rendered_chunk_count, instance_buffer_size);

		let buffer = buffer::Buffer::create_gpu(
			Some(format!("RenderVoxel.InstanceBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::VERTEX_BUFFER,
			instance_buffer_size,
			None,
		)?;

		Ok(Self {
			chunk_cache,
			buffer,
			instances: Vec::with_capacity(max_rendered_instances),
			ids: HashMap::new(),
		})
	}

	pub fn buffer(&self) -> &Arc<buffer::Buffer> {
		&self.buffer
	}

	pub fn ids(&self) -> &HashMap<block::LookupId, Vec<usize>> {
		&self.ids
	}

	pub fn prerecord_update(
		&mut self,
		render_chain: &RenderChain,
	) -> Result<Vec<Arc<command::Semaphore>>, AnyError> {
		let mut pending_gpu_signals = Vec::new();

		let (pending, removed) = match self.chunk_cache.write() {
			Ok(mut cache) => cache.take_pending(),
			_ => return Ok(pending_gpu_signals),
		};

		let mut changed_instances = false;
		if !removed.is_empty() {
			log::debug!("Remove voxel instance chunks: {:?}", removed.len());
		}
		if !pending.is_empty() {
			log::debug!("Add voxel instance chunks: {:?}", pending.len());
			for arc_chunk in pending.into_iter() {
				let chunk = arc_chunk.read().unwrap();
				let chunk_coord = chunk.coordinate();
				for (point, block_id) in chunk.block_ids().iter() {
					if let Some(asset_id) = block::Lookup::lookup_id(*block_id) {
						log::debug!("<{}, {}, {}> = {}", point.x, point.y, point.z, asset_id);
					}
					
					if !self.ids.contains_key(&block_id) {
						self.ids.insert(*block_id, Vec::new());
					}
					self.ids.get_mut(&block_id).unwrap().push(self.instances.len());
					
					changed_instances = true;
					self.instances.push(
						Instance {
							chunk_coordinate: Vector3::new(
								chunk_coord.x as f32,
								chunk_coord.y as f32,
								chunk_coord.z as f32,
							).into(),
							model_matrix: Translation3::new(
								point.x as f32,
								point.y as f32,
								point.z as f32,
							).to_homogeneous().into(),
							instance_flags: instance::Flags {
								faces: EnumSet::all(),
							}
							.into(),
						}
					);
				}
			}
		}

		if changed_instances {
			graphics::TaskGpuCopy::new(
				self.buffer.wrap_name(|v| format!("Write({})", v)),
				&render_chain,
			)?
			.begin()?
			.stage(&self.instances[..])?
			.copy_stage_to_buffer(&self.buffer)
			.end()?
			.add_signal_to(&mut pending_gpu_signals)
			.send_to(task::sender());
		}

		Ok(pending_gpu_signals)
	}
}
