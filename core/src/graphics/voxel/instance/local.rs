use crate::{
	block,
	graphics::voxel::{
		instance::{
			category::{self, Category},
			Instance, RangeSet,
		},
		model, Face,
	},
};
use engine::math::nalgebra::Point3;
use enumset::EnumSet;
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Weak},
};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum IdPhase {
	Active,
	Inactive,
}

pub struct IntegratedBuffer {
	model_cache: Weak<model::Cache>,
	/// The ordered list of all instances in the buffer.
	/// Some of these may be garbage data.
	/// USe `category_keys` and `categories` to determine which instances belong to which category.
	instances: Vec<Instance>,
	block_type_count: block::LookupId,
	/// The ordered list of categories, which determine what block-type each item in `instances` is.
	categories: Vec<Category>,
	/// Mapping of block::Point to the block-type it is.
	/// If it is not in this mapping, the point is either empty (air), or exists in `inactive_instances` as a block without rendered faces.
	active_points: HashMap<Point3<i64>, HashMap<Point3<i8>, (block::LookupId, usize)>>,
	/// Mapping of block::Point to its instance, if the point cannot render any faces.
	/// Does not include points which are empty (air).
	inactive_points: HashMap<Point3<i64>, HashMap<Point3<i8>, (block::LookupId, Instance)>>,
	changed_ranges: RangeSet,
}

impl IntegratedBuffer {
	pub fn new(instance_capacity: usize, model_cache: Weak<model::Cache>) -> Self {
		let block_type_count = block::Lookup::get().unwrap().count();
		let categories = Self::create_categories(block_type_count, instance_capacity);
		let instances = vec![Instance::default(); instance_capacity];
		Self {
			model_cache,
			instances,
			block_type_count,
			categories,
			active_points: HashMap::new(),
			inactive_points: HashMap::new(),
			changed_ranges: RangeSet::default(),
		}
	}

	fn create_categories(
		block_type_count: block::LookupId,
		instance_capacity: usize,
	) -> Vec<Category> {
		let mut values = Vec::with_capacity(block_type_count + 1);
		for id in 0..block_type_count {
			values.push(Category::new(Some(id), 0));
		}
		values.push(Category::new(None, instance_capacity));
		values
	}
}

impl IntegratedBuffer {
	#[profiling::function]
	pub fn take_changed_ranges(&mut self) -> Option<(Vec<std::ops::Range<usize>>, usize)> {
		match self.changed_ranges.is_empty() {
			true => None,
			false => Some(self.changed_ranges.take()),
		}
	}

	pub fn instances(&self) -> &Vec<Instance> {
		&self.instances
	}

	pub fn get_categories(&self) -> &Vec<Category> {
		&self.categories
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

		let mut points = HashSet::with_capacity(block_ids.len());
		for (point, block_id) in block_ids.iter() {
			let point = block::Point::new(*chunk, point.cast::<i8>());
			self.insert_inactive(&point, *block_id, Instance::from(&point, EnumSet::empty()));
			points.insert(point);
		}
		self.update_faces(points);
	}

	pub fn remove_chunk(&mut self, coord: &Point3<i64>) {
		if let Some(active_points) = self.active_points.get(&coord).cloned() {
			for (point_offset, (block_id, _instance_idx)) in active_points.into_iter() {
				self.remove(&block::Point::new(*coord, point_offset), block_id);
			}
			assert_eq!(self.active_points.get(&coord).unwrap().len(), 0);
		}

		let _ = self.active_points.remove(&coord);
		let _ = self.inactive_points.remove(&coord);
	}

	pub fn set_id_for(&mut self, point: &block::Point, id: Option<block::LookupId>) {
		match self.get_block_id(&point) {
			Some((_phase, prev_block_id)) => match id {
				Some(next_block_id) => {
					self.change_id(&point, prev_block_id, next_block_id);
				}
				None => {
					self.remove(&point, prev_block_id);
				}
			},
			None => {
				if let Some(id) = id {
					self.insert(&point, id);
				}
			}
		}
	}
}

impl IntegratedBuffer {
	fn max_block_id(&self) -> block::LookupId {
		self.block_type_count - 1
	}

	fn get_category_idx(&self, key: category::Key) -> usize {
		match key {
			category::Key::Id(block_id) => block_id,
			category::Key::Unallocated => self.block_type_count,
		}
	}

	fn get_category(&self, key: category::Key) -> &Category {
		let idx = self.get_category_idx(key);
		&self.categories[idx]
	}

	fn get_category_mut(&mut self, key: category::Key) -> &mut Category {
		let idx = self.get_category_idx(key);
		&mut self.categories[idx]
	}

	fn insert(&mut self, point: &block::Point, next_id: block::LookupId) {
		self.insert_inactive(&point, next_id, Instance::from(&point, EnumSet::empty()));
		self.update_faces(HashSet::from([*point]));
	}

	fn change_id(
		&mut self,
		point: &block::Point,
		prev_id: block::LookupId,
		next_id: block::LookupId,
	) {
		self.change_category(
			&point,
			category::Key::Id(prev_id),
			category::Key::Id(next_id),
		);
	}

	/// Deallocates the instance data and removes all reference to the point from the metadata (active AND inactive).
	fn remove(&mut self, point: &block::Point, prev_id: block::LookupId) {
		if let Some(idx) = self.change_category(
			&point,
			category::Key::Id(prev_id),
			category::Key::Unallocated,
		) {
			self.instances[idx] = Instance::default();
			self.changed_ranges.insert(idx);
		}
	}

	fn insert_inactive(&mut self, point: &block::Point, id: block::LookupId, instance: Instance) {
		if !self.inactive_points.contains_key(point.chunk()) {
			self.inactive_points.insert(*point.chunk(), HashMap::new());
		}
		let inactive_chunk_points = self.inactive_points.get_mut(point.chunk()).unwrap();
		inactive_chunk_points.insert(*point.offset(), (id, instance));
	}

	fn get_block_id(&self, point: &block::Point) -> Option<(IdPhase, block::LookupId)> {
		if let Some(chunk_points) = self.inactive_points.get(&point.chunk()) {
			if let Some((id, _instance)) = chunk_points.get(&point.offset()) {
				return Some((IdPhase::Inactive, *id));
			}
		}
		if let Some(chunk_points) = self.active_points.get(&point.chunk()) {
			if let Some((id, _instance_idx)) = chunk_points.get(&point.offset()) {
				return Some((IdPhase::Active, *id));
			}
		}
		None
	}

	fn change_category(
		&mut self,
		point: &block::Point,
		start: category::Key,
		destination: category::Key,
	) -> Option<usize> {
		let path = match category::Key::new_path(start, destination, self.max_block_id()) {
			Some(path) => path,
			None => return None,
		};

		let mut instance_idx = match start {
			category::Key::Unallocated => self.get_category(start).start(),
			_ => match self.active_points.get_mut(&point.chunk()) {
				Some(chunk_points) => match chunk_points.remove(&point.offset()) {
					Some((_id, instance_idx)) => instance_idx,
					None => unimplemented!(),
				},
				None => unimplemented!(),
			},
		};

		let direction = category::Direction::from(&start, &destination);

		let mut prev_key = start;
		for next_key in path.into_iter() {
			if next_key != start {
				for (key, operation) in direction.operations(prev_key, next_key).into_iter() {
					let category = self.get_category_mut(key);
					category.apply(operation);
				}
			}

			if next_key != destination {
				let target_idx = self
					.get_category(next_key)
					.index_at_position(direction.target_position());

				let target_point = self.instances[target_idx].point();
				self.set_point_index(&target_point, instance_idx);

				self.swap_instances(&mut instance_idx, target_idx);
			}

			prev_key = next_key;
		}

		if let category::Key::Id(block_id) = destination {
			if !self.active_points.contains_key(&point.chunk()) {
				self.active_points.insert(*point.chunk(), HashMap::new());
			}
			let chunk_points = self.active_points.get_mut(&point.chunk()).unwrap();
			let _ = chunk_points.insert(*point.offset(), (block_id, instance_idx));
		}

		Some(instance_idx)
	}

	fn set_point_index(&mut self, point: &block::Point, idx: usize) {
		if let Some(chunk_points) = self.active_points.get_mut(&point.chunk()) {
			if let Some((_id, instance_idx)) = chunk_points.get_mut(&point.offset()) {
				*instance_idx = idx;
			}
		}
	}

	fn swap_instances(&mut self, a: &mut usize, b: usize) {
		if *a == b {
			return;
		}
		self.instances.swap(*a, b);
		self.changed_ranges.insert(*a);
		self.changed_ranges.insert(b);
		*a = b;
	}

	fn get_instance_mut(
		&mut self,
		point: &block::Point,
		phase: IdPhase,
	) -> Option<(Option<usize>, &mut Instance)> {
		match phase {
			IdPhase::Active => match self.active_points.get_mut(&point.chunk()) {
				Some(chunk_points) => match chunk_points.get_mut(&point.offset()) {
					Some((_id, instance_idx)) => match self.instances.get_mut(*instance_idx) {
						Some(instance) => Some((Some(*instance_idx), instance)),
						None => None,
					},
					None => None,
				},
				None => None,
			},
			IdPhase::Inactive => match self.inactive_points.get_mut(&point.chunk()) {
				Some(chunk_points) => match chunk_points.get_mut(&point.offset()) {
					Some((_id, instance)) => Some((None, instance)),
					None => None,
				},
				None => None,
			},
		}
	}

	#[profiling::function]
	fn update_faces(&mut self, points: HashSet<block::Point>) {
		let model_cache = self.model_cache.upgrade().unwrap();

		let mut changes = Vec::new();

		let all_faces = EnumSet::<Face>::all();
		// For each point in the set, check all of its faces (and update its neighbor if necessary)
		for &primary_point in points.iter() {
			profiling::scope!("gather-faces", &format!("{}", primary_point));

			// Gather the block-type of each voxel on a given face of the point
			let mut face_ids = Vec::with_capacity(all_faces.len());
			for primary_point_face in all_faces.iter() {
				profiling::scope!("face", &format!("{}", primary_point_face));
				// Get the block::Point of the block on that face of the primary point
				let secondary_point = primary_point + primary_point_face.direction();
				// And get the block-type of that adjacent point
				let secondary_point_id = self.get_block_id(&secondary_point);
				// Save off the adjacent block information
				face_ids.push((primary_point_face, secondary_point));
				// The secondary point could be empty (air). If it is, then it doesnt have a block-id.
				if let Some((secondary_point_phase, secondary_point_id)) = secondary_point_id {
					// If the adjacent point is not a primary point, the face that
					// is adjacent to the primary point should also be updated.
					// If it IS a primary point, it has either already been
					// visited or will be visited shortly.
					if !points.contains(&secondary_point) {
						let secondary_point_face = primary_point_face.inverse();
						let desired_phase = self.recalculate_faces(
							secondary_point,
							secondary_point_phase,
							secondary_point_id,
							vec![(secondary_point_face, primary_point)],
							&model_cache,
						);
						if desired_phase != secondary_point_phase {
							changes.push((secondary_point, secondary_point_phase, desired_phase));
						}
					}
				}
			}
			// Update the faces for this primary point
			if let Some((primary_point_phase, primary_point_id)) = self.get_block_id(&primary_point)
			{
				let desired_phase = self.recalculate_faces(
					primary_point,
					primary_point_phase,
					primary_point_id,
					face_ids,
					&model_cache,
				);
				if desired_phase != primary_point_phase {
					changes.push((primary_point, primary_point_phase, desired_phase));
				}
			}
		}

		{
			profiling::scope!("apply-phase-changes");
			for (point, phase, desired_phase) in changes.into_iter() {
				self.change_phase(&point, phase, desired_phase);
			}
		}
	}

	fn recalculate_faces(
		&mut self,
		point: block::Point,
		phase: IdPhase,
		id: block::LookupId,
		faces: Vec<(Face, block::Point)>,
		model_cache: &Arc<model::Cache>,
	) -> IdPhase {
		profiling::scope!(
			"recalculate_faces",
			&format!("point:{} face-count:{}", point, faces.len())
		);

		let faces = faces
			.into_iter()
			.map(|(face, adj_point)| (face, self.get_block_id(&adj_point)))
			.collect::<Vec<_>>();

		let mut desired_phase = phase;
		if let Some((idx, instance)) = self.get_instance_mut(&point, phase) {
			let mut point_faces = instance.faces();
			for (face, block_id) in faces.into_iter() {
				let face_is_enabled = match block_id {
					// Block doesnt exist at this point (its air/empty) or the chunk isn't loaded.
					None => true,
					Some((_phase, block_id)) => match model_cache.get(&block_id) {
						// Found a model, can base face visibility based on if the model is fully-opaque
						Some((model, _, _)) => {
							// The other block is opaque, our face should be shown.
							if model.is_opaque() {
								false
							}
							// The other block is not opaque, show our face only if the types are not the same.
							// i.e. two adjacent glass blocks should not show their touching faces
							else {
								block_id != id
							}
						}
						// No model matches the id... x_x
						None => unimplemented!(),
					},
				};

				if face_is_enabled {
					point_faces.insert(face);
				} else {
					point_faces.remove(face);
				}
			}
			if instance.faces() != point_faces {
				instance.set_faces(point_faces);
				if let Some(idx) = idx {
					self.changed_ranges.insert(idx);
				}
			}

			// The desired phase of the instance is based on
			// if there are any faces that should be rendered.
			desired_phase = if point_faces.is_empty() {
				IdPhase::Inactive
			} else {
				IdPhase::Active
			};
		}

		desired_phase
	}

	fn change_phase(&mut self, point: &block::Point, prev: IdPhase, next: IdPhase) {
		profiling::scope!("change_phase", &format!("{} {:?}->{:?}", point, prev, next));
		match (prev, next) {
			// Deactivating a block, time to remove it from the buffered data.
			(IdPhase::Active, IdPhase::Inactive) => {
				let (id, instance_idx) = match self.active_points.get_mut(&point.chunk()) {
					Some(chunk_points) => match chunk_points.get(&point.offset()) {
						Some((id, instance_idx)) => (*id, *instance_idx),
						None => return,
					},
					None => return,
				};
				// Clone the instance out of the buffered data
				let instance = self.instances.get(instance_idx).unwrap().clone();

				// Move the instance data to the unallocated section
				self.remove(&point, id);

				// Insert the point and instance into the inactive thunk
				if !self.inactive_points.contains_key(&point.chunk()) {
					self.inactive_points.insert(*point.chunk(), HashMap::new());
				}
				if let Some(chunk_points) = self.inactive_points.get_mut(&point.chunk()) {
					chunk_points.insert(*point.offset(), (id, instance));
				}
			}
			(IdPhase::Inactive, IdPhase::Active) => {
				// The voxel should be rendered! (at least 1 face).
				// Extract from inactive thunk
				let (id, instance) = match self.inactive_points.get_mut(&point.chunk()) {
					Some(chunk_points) => match chunk_points.remove(&point.offset()) {
						Some((id, instance)) => (id, instance),
						None => return,
					},
					None => return,
				};

				let instance_idx = self
					.change_category(&point, category::Key::Unallocated, category::Key::Id(id))
					.unwrap();
				match self.instances.get_mut(instance_idx) {
					Some(target) => {
						*target = instance;
						self.changed_ranges.insert(instance_idx);
					}
					None => return,
				}
			}
			_ => unimplemented!(),
		}
	}
}
