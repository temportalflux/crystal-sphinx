use crate::{block, graphics::voxel::instance::Instance};
use engine::math::nalgebra::Point3;
use std::collections::{HashMap, HashSet};

/// Wrapper struct containing the chunk coordinate and offset within the chunk for a given block.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BlockPoint(Point3<i64>, Point3<usize>);

/// The description of the local block instance data in the local-memory of the program.
/// This data is mutable between frames. When the [`instance buffer`](super::Buffer)
/// copies instance data to the GPU buffer, this description is copied to the [`submitted description`](super::SubmittedDescription).
pub struct Description {
	// NOTES:
	// - Each block-id should have a continugous section of the data written to buffer such that it has a start index and a count.
	// - There are going to be instances for each block-id which have NO FACES to render,
	//   these should just not be copied to the instance buffer at all (to save on space).
	// IMPROVEMENTS:
	// - There is a world in which we use strategic swapping of elements to reduce the amount of bytes written on change.
	//   Whenever an instances changes from one type to another (or to empty or completely hidden), it would be shuffled along the
	//   instances vec and the start indicies of each categoryy would be updated.
	//   For the sake of getting the system up and running though, thats an optimization that can be figured out at a later time.
	categories: HashMap<block::LookupId, Category>,
	chunks: HashMap<Point3<i64>, ChunkDesc>,
	has_changes: bool,
}

#[derive(Debug)]
enum InstanceReference {
	Active(usize),
	Disabled(usize),
}
impl std::ops::Deref for InstanceReference {
	type Target = usize;
	fn deref(&self) -> &Self::Target {
		match &self {
			Self::Active(idx) => idx,
			Self::Disabled(idx) => idx,
		}
	}
}
impl std::ops::DerefMut for InstanceReference {
	fn deref_mut(&mut self) -> &mut Self::Target {
		match self {
			Self::Active(idx) => idx,
			Self::Disabled(idx) => idx,
		}
	}
}

#[derive(Default)]
pub struct Category {
	/// Instances which have at least 1 face adjacent to a non-opague block (or no block).
	/// These instances are written to the GPU and rendered.
	active_instances: Vec<Instance>,
	active_points: Vec<BlockPoint>,
	/// Instances which have all faces disabled (because each faces is adjacent to an opague block).
	disabled_instances: Vec<Instance>,
	disabled_points: Vec<BlockPoint>,
}

#[derive(Default)]
struct ChunkDesc {
	/// Lookup instance index by block offset point.
	blocks: HashMap<Point3<usize>, ChunkBlock>,
}

struct ChunkBlock {
	id: block::LookupId,
	redirector: InstanceReference,
}

impl Description {
	pub fn new() -> Self {
		let mut categories = HashMap::new();
		for id in Self::ordered_ids() {
			categories.insert(id, Category::default());
		}
		Self {
			categories,
			chunks: HashMap::new(),
			has_changes: false,
		}
	}

	/// Mapping of [`lookup id`](block::LookupId) to the asset id of the block.
	/// The instances follow this order when writing instances to the submitted description.
	pub(crate) fn ordered_ids() -> std::ops::Range<usize> {
		0..block::Lookup::get().unwrap().count()
	}

	pub fn get_instances(&self, id: &block::LookupId) -> &Vec<Instance> {
		&self.categories.get(&id).unwrap().active_instances
	}

	pub fn take_has_changes(&mut self) -> bool {
		let changed = self.has_changes;
		self.has_changes = false;
		changed
	}

	pub fn set_block_id(
		&mut self,
		chunk: &Point3<i64>,
		offset: &Point3<usize>,
		id: Option<block::LookupId>,
	) {
		if !self.chunks.contains_key(&chunk) {
			self.chunks.insert(chunk.clone(), ChunkDesc::default());
		}

		// Early returns if the desired change is a no-op,
		// otherwise returns the remove block description (if it exists).
		// Will be none if there was no block at that coordinate.
		let removed_block_desc = {
			let chunk_desc = self.chunks.get_mut(&chunk).unwrap();

			// If the current block id of the instance at the point is already the desired id, then this function is a NO-OP.
			match (chunk_desc.blocks.get(&offset), id.as_ref()) {
				(Some(ChunkBlock { id, .. }), Some(desired_id)) if desired_id == id => return,
				(None, None) => return,
				// Resulting options:
				// None -> Some(x): air becoming block
				// Some(x) -> None: block becoming air
				// Some(x) -> Some(y): block replacement
				_ => {}
			}

			chunk_desc.blocks.remove(&offset)
		};

		let instance = if let Some(block_desc) = removed_block_desc {
			Some(self.remove(&block_desc))
		} else {
			None
		};

		let block_id = match id {
			// Block is being removed, we can return now and drop the instance.
			None => return,
			Some(id) => id,
		};
		let instance = instance.unwrap_or(Instance::from(&chunk, &offset));
		let point = BlockPoint(*chunk, *offset);
		let redirector = self.insert(&block_id, point, instance);

		{
			let chunk_desc = self.chunks.get_mut(&chunk).unwrap();
			chunk_desc.blocks.insert(
				offset.clone(),
				ChunkBlock {
					id: block_id,
					redirector,
				},
			);
		}

		self.update_proximity(&HashSet::from([point]));
	}

	fn take_first_block_in_chunk(
		&mut self,
		coord: &Point3<i64>,
	) -> Option<(Point3<usize>, ChunkBlock)> {
		if let Some(chunk_desc) = self.chunks.get_mut(&coord) {
			if let Some(offset) = chunk_desc.blocks.keys().next().cloned() {
				return chunk_desc.blocks.remove(&offset).map(|desc| (offset, desc));
			}
		}
		None
	}

	pub fn remove_chunk(&mut self, coord: &Point3<i64>) {
		let block_count = match self.chunks.get(&coord) {
			Some(chunk_desc) => chunk_desc.blocks.len(),
			None => return,
		};

		let mut offsets = HashSet::with_capacity(block_count);
		while let Some((offset, block_desc)) = self.take_first_block_in_chunk(&coord) {
			let _instance = self.remove(&block_desc);
			offsets.insert(BlockPoint(*coord, offset));
		}

		let chunk_desc = self.chunks.remove(&coord);
		assert!(chunk_desc.is_some());
		assert!(chunk_desc.unwrap().blocks.is_empty());

		self.update_proximity(&offsets);
	}

	fn insert(
		&mut self,
		block_id: &block::LookupId,
		point: BlockPoint,
		instance: Instance,
	) -> InstanceReference {
		let category = self.categories.get_mut(&block_id).unwrap();
		let idx = category.active_instances.len();

		category.active_instances.push(instance);
		category.active_points.push(point);
		self.has_changes = true;

		InstanceReference::Active(idx)
	}

	fn remove(&mut self, desc: &ChunkBlock) -> Instance {
		let category = self.categories.get_mut(&desc.id).unwrap();

		let (idx, swapped_point, removed_instance) = match desc.redirector {
			InstanceReference::Active(idx) => {
				let swapped_point = category.active_points.last().cloned();
				let _removed_point = category.active_points.swap_remove(idx);
				let removed_instance = category.active_instances.swap_remove(idx);
				(idx, swapped_point, removed_instance)
			}
			InstanceReference::Disabled(idx) => {
				let swapped_point = category.disabled_points.last().cloned();
				let _removed_point = category.disabled_points.swap_remove(idx);
				let removed_instance = category.disabled_instances.swap_remove(idx);
				(idx, swapped_point, removed_instance)
			}
		};

		// The block that was at the end of the vec is now at `idx`,
		// so the redirector in the chunk desc's needs to be updated.
		if let Some(swapped_point) = swapped_point {
			let swapped_chunk = self.chunks.get_mut(&swapped_point.0);
			let swapped_block = swapped_chunk
				.map(|c| c.blocks.get_mut(&swapped_point.1))
				.flatten();
			if let Some(swapped_block) = swapped_block {
				*swapped_block.redirector = idx;
			}
		}

		self.has_changes = true;

		removed_instance
	}

	fn update_proximity(&mut self, points: &HashSet<BlockPoint>) {
		// TODO: Update the face data for all blocks adjacent to the provided points (and the points themselves)
	}
}
