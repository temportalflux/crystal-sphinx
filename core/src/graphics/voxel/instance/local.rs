use crate::{
	block,
	graphics::voxel::{instance::Instance, model, Face},
	world::chunk,
};
use engine::math::nalgebra::{Point3, Vector3};
use enumset::EnumSet;
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Weak},
};

/// Wrapper struct containing the chunk coordinate and offset within the chunk for a given block.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BlockPoint(Point3<i64>, Point3<i8>);
impl std::fmt::Debug for BlockPoint {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for BlockPoint {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"<{}'{}, {}'{}, {}'{}>",
			self.0.x, self.1.x, self.0.y, self.1.y, self.0.z, self.1.z,
		)
	}
}
impl std::ops::Add<Vector3<i8>> for BlockPoint {
	type Output = Self;
	fn add(mut self, other: Vector3<i8>) -> Self::Output {
		self.1 += other;
		self.align();
		self
	}
}
impl std::ops::Sub<BlockPoint> for BlockPoint {
	type Output = Self;
	fn sub(self, rhs: BlockPoint) -> Self::Output {
		Self(self.0 - rhs.0.coords, self.1 - rhs.1.coords)
	}
}
impl BlockPoint {
	pub fn new(chunk: Point3<i64>, offset: Point3<i8>) -> Self {
		let mut point = Self(chunk, offset);
		point.align();
		point
	}

	fn align(&mut self) {
		let chunk = &mut self.0;
		let offset = &mut self.1;
		let size = chunk::SIZE_I;
		for i in 0..size.len() {
			let size = size[i] as i8;
			if offset[i] < 0 {
				let amount = (offset[i].abs() / size) + 1;
				chunk[i] -= amount as i64;
				offset[i] += amount * size;
			}
			if offset[i] >= size {
				let amount = offset[i].abs() / size;
				chunk[i] += amount as i64;
				offset[i] -= amount * size;
			}
		}
	}

	fn chunk(&self) -> &Point3<i64> {
		&self.0
	}

	fn offset(&self) -> &Point3<i8> {
		&self.1
	}
}

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
	model_cache: Weak<model::Cache>,
	categories: HashMap<block::LookupId, Category>,
	chunks: HashMap<Point3<i64>, ChunkDesc>,
	has_changes: bool,
}

#[derive(Debug, Clone, Copy)]
enum InstanceReference {
	Active(usize),
	Disabled(usize),
}
impl InstanceReference {
	fn is_active(&self) -> bool {
		match self {
			Self::Active(_) => true,
			Self::Disabled(_) => false,
		}
	}
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
	blocks: HashMap<Point3<i8>, ChunkBlock>,
}

#[derive(Clone, Debug)]
struct ChunkBlock {
	id: block::LookupId,
	redirector: InstanceReference,
}

#[derive(Debug, Clone, Copy)]
enum BlockDescId {
	ChunkNotLoaded,
	Empty,
	Id(block::LookupId),
}

impl Description {
	pub fn new(model_cache: Weak<model::Cache>) -> Self {
		let mut categories = HashMap::new();
		for id in Self::ordered_ids() {
			categories.insert(id, Category::default());
		}
		Self {
			model_cache,
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

	fn get_instance_redirector(&self, point: &BlockPoint) -> Option<ChunkBlock> {
		match self.chunks.get(point.chunk()) {
			Some(chunk_desc) => chunk_desc.blocks.get(point.offset()).cloned(),
			None => None,
		}
	}

	fn get_instance_mut(&mut self, block_desc: &ChunkBlock) -> Option<&mut Instance> {
		let category = self.categories.get_mut(&block_desc.id).unwrap();
		match block_desc.redirector {
			InstanceReference::Active(idx) => category.active_instances.get_mut(idx),
			InstanceReference::Disabled(idx) => category.disabled_instances.get_mut(idx),
		}
	}

	fn get_block_id(&self, point: &BlockPoint) -> BlockDescId {
		match self.chunks.get(point.chunk()) {
			Some(chunk_desc) => match chunk_desc.blocks.get(point.offset()) {
				Some(desc) => BlockDescId::Id(desc.id),
				None => BlockDescId::Empty,
			},
			None => BlockDescId::ChunkNotLoaded,
		}
	}

	pub fn insert_chunk(
		&mut self,
		chunk: &Point3<i64>,
		block_ids: &HashMap<Point3<usize>, block::LookupId>,
	) {
		profiling::scope!(
			"insert_chunk",
			&format!("<{}, {}, {}>", chunk.x, chunk.y, chunk.z)
		);

		self.chunks.insert(*chunk, ChunkDesc::default());

		let mut points = HashSet::with_capacity(chunk::DIAMETER.pow(3));
		for (point, block_id) in block_ids.iter() {
			points.insert(BlockPoint::new(*chunk, point.cast::<i8>()));
			self.set_block_id(&chunk, &point.cast::<i8>(), Some(*block_id), false);
		}
		if !points.is_empty() {
			self.update_proximity(points);
		}
	}

	pub fn set_block_id(
		&mut self,
		chunk: &Point3<i64>,
		offset: &Point3<i8>,
		id: Option<block::LookupId>,
		update_neighbors: bool,
	) {
		assert!(self.chunks.contains_key(&chunk));
		let point = BlockPoint::new(*chunk, *offset);

		let _scope_tag = format!("{} {:?}", point, id);
		profiling::scope!("set_block_id", _scope_tag.as_str());

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

		let (point, instance) = if let Some(block_desc) = removed_block_desc {
			self.remove(&block_desc)
		} else {
			(point, Instance::from(&chunk, &offset))
		};

		let block_id = match id {
			// Block is being removed, we can return now and drop the instance.
			None => return,
			Some(id) => id,
		};
		self.insert(&block_id, point, instance, true);

		if update_neighbors {
			self.update_proximity(HashSet::from([point]));
		}
	}

	fn take_first_block_in_chunk(
		&mut self,
		coord: &Point3<i64>,
	) -> Option<(Point3<i8>, ChunkBlock)> {
		if let Some(chunk_desc) = self.chunks.get_mut(&coord) {
			if let Some(offset) = chunk_desc.blocks.keys().next().cloned() {
				return chunk_desc.blocks.remove(&offset).map(|desc| (offset, desc));
			}
		}
		None
	}

	pub fn remove_chunk(&mut self, coord: &Point3<i64>) {
		let _scope_tag = format!("<{}, {}, {}>", coord.x, coord.y, coord.z);
		profiling::scope!("remove_chunk", _scope_tag.as_str());

		let block_count = match self.chunks.get(&coord) {
			Some(chunk_desc) => chunk_desc.blocks.len(),
			None => return,
		};

		let mut offsets = HashSet::with_capacity(block_count);
		while let Some((offset, block_desc)) = self.take_first_block_in_chunk(&coord) {
			let _instance = self.remove(&block_desc);
			offsets.insert(BlockPoint::new(*coord, offset));
		}

		let chunk_desc = self.chunks.remove(&coord);
		assert!(chunk_desc.is_some());
		assert!(chunk_desc.unwrap().blocks.is_empty());

		self.update_proximity(offsets);
	}

	fn insert(
		&mut self,
		block_id: &block::LookupId,
		point: BlockPoint,
		instance: Instance,
		is_active: bool,
	) {
		let category = self.categories.get_mut(&block_id).unwrap();
		self.has_changes = true;

		let redirector = match is_active {
			true => {
				let idx = category.active_instances.len();
				category.active_instances.push(instance);
				category.active_points.push(point);
				InstanceReference::Active(idx)
			}
			false => {
				let idx = category.disabled_instances.len();
				category.disabled_instances.push(instance);
				category.disabled_points.push(point);
				InstanceReference::Disabled(idx)
			}
		};

		let chunk_desc = self.chunks.get_mut(point.chunk()).unwrap();
		chunk_desc.blocks.insert(
			point.offset().clone(),
			ChunkBlock {
				id: *block_id,
				redirector,
			},
		);
	}

	fn remove(&mut self, desc: &ChunkBlock) -> (BlockPoint, Instance) {
		let category = self.categories.get_mut(&desc.id).unwrap();

		let (idx, swapped_point, removed) = match desc.redirector {
			InstanceReference::Active(idx) => {
				let swapped_point = category.active_points.last().cloned();
				let removed_point = category.active_points.swap_remove(idx);
				let removed_instance = category.active_instances.swap_remove(idx);
				(idx, swapped_point, (removed_point, removed_instance))
			}
			InstanceReference::Disabled(idx) => {
				let swapped_point = category.disabled_points.last().cloned();
				let removed_point = category.disabled_points.swap_remove(idx);
				let removed_instance = category.disabled_instances.swap_remove(idx);
				(idx, swapped_point, (removed_point, removed_instance))
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

		removed
	}

	fn update_proximity(&mut self, points: HashSet<BlockPoint>) {
		profiling::scope!("update_proximity");

		let model_cache = self.model_cache.upgrade().unwrap();

		let all_faces = EnumSet::<Face>::all();
		for &point1 in points.iter() {
			profiling::scope!("update-faces", &format!("{}", point1));
			let point1_id = self.get_block_id(&point1);
			let mut face_ids = Vec::with_capacity(all_faces.len());
			for point1_face in all_faces.iter() {
				profiling::scope!("face", &format!("{}", point1_face));
				let point2 = point1 + point1_face.direction();
				let point2_id = self.get_block_id(&point2);
				face_ids.push((point1_face, point2, point2_id));
				if !points.contains(&point2) {
					let point2_face = point1_face.inverse();
					if self.update_instance_flags(
						point2,
						vec![(point2_face, point1, point1_id)],
						&model_cache,
					) {
						self.has_changes = true;
					}
				}
			}
			if self.update_instance_flags(point1, face_ids, &model_cache) {
				self.has_changes = true;
			}
		}
	}

	fn update_instance_flags(
		&mut self,
		point: BlockPoint,
		faces: Vec<(Face, BlockPoint, BlockDescId)>,
		model_cache: &Arc<model::Cache>,
	) -> bool {
		let mut has_changed = false;
		if let Some(block_desc) = self.get_instance_redirector(&point) {
			profiling::scope!("update_instance_flags", &format!("{}", point));
			let should_be_enabled = match self.get_instance_mut(&block_desc) {
				Some(instance) => {
					let mut point_faces = instance.faces();
					for (face, _, desc_id) in faces.into_iter() {
						let face_is_enabled = match desc_id {
							// If the point being processed isn't loaded, then it was
							// some point adjacent to the original given list
							// that we dont care about right now.
							BlockDescId::ChunkNotLoaded => true,
							// Block doesnt exist at this point, its air/empty.
							BlockDescId::Empty => true,
							BlockDescId::Id(id) => match model_cache.get(&id) {
								Some((model, _, _)) => !model.is_opaque(),
								None => true,
							},
						};

						if face_is_enabled {
							point_faces.insert(face);
						} else {
							point_faces.remove(face);
						}
					}
					if instance.faces() != point_faces {
						has_changed = true;
						instance.set_faces(point_faces);
					}
					!point_faces.is_empty()
				}
				None => false,
			};
			if block_desc.redirector.is_active() != should_be_enabled {
				let (point, instance) = self.remove(&block_desc);
				self.insert(&block_desc.id, point, instance, should_be_enabled);
				has_changed = true;
			}
		}
		has_changed
	}
}
