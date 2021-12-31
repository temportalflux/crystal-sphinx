use crate::{
	graphics::voxel::instance::{local, submitted, Instance},
	world::chunk::{self, ArcLockClientCache},
};
use engine::{
	graphics::{command, RenderChain},
	utility::AnyError,
};
use std::sync::Arc;

/// Controls the instance buffer data for rendering voxels.
/// Keeps track of what chunks and blocks are old and updates the instances accordingly.
pub struct Buffer {
	chunk_cache: ArcLockClientCache,
	local_description: local::Description,
	submitted_description: submitted::Description,
}

impl Buffer {
	pub fn new(
		render_chain: &RenderChain,
		chunk_cache: ArcLockClientCache,
	) -> Result<Self, AnyError> {
		// TODO: Get this value from settings
		let render_radius = 5;
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

		log::debug!("Initializing voxel instance buffer: chunk_radius={} total_chunk_count={}, buffer_size={}(bytes)", render_radius, rendered_chunk_count, instance_buffer_size);

		let local_description = local::Description::new();
		let submitted_description =
			submitted::Description::new(&render_chain, instance_buffer_size)?;

		Ok(Self {
			chunk_cache,
			local_description,
			submitted_description,
		})
	}

	pub fn submitted(&self) -> &submitted::Description {
		&self.submitted_description
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

		if !removed.is_empty() {
			for coord in removed.into_iter() {
				self.local_description.remove_chunk(&coord);
			}
		}
		if !pending.is_empty() {
			for arc_chunk in pending.into_iter() {
				let chunk = arc_chunk.read().unwrap();
				let coord = chunk.coordinate();
				for (point, block_id) in chunk.block_ids().iter() {
					self.local_description
						.set_block_id(&coord, point, Some(*block_id));
				}
			}
		}

		if self.local_description.take_has_changes() {
			self.submitted_description.submit(
				&self.local_description,
				&render_chain,
				&mut pending_gpu_signals,
			)?;
		}

		Ok(pending_gpu_signals)
	}
}
