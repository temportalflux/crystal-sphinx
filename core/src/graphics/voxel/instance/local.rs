use crate::{
	block,
	graphics::voxel::{
		instance::{Category, Instance},
		model, Face,
	},
};
use engine::math::nalgebra::Point3;
use enumset::EnumSet;
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CategoryKey {
	/// A category for a specific block-type.
	Id(block::LookupId),
	/// A unique category full of garbage data.
	/// Holds the segment from which new voxel instances are allocated from.
	Unallocated,
}

impl CategoryKey {
	/// Determines the order of segments that need to be traversed in order to move an instance from `start` to `destination`.
	/// If `start == destination`, the returned path will be None, otherwise the resulting path
	/// will always contain `start` as the first value and `destingation` as the last value.
	fn new_path(start: Self, destination: Self, max_id: block::LookupId) -> Option<Vec<Self>> {
		use std::cmp::Ordering;
		if start == destination {
			return None;
		}
		let mut path = Vec::new();
		let mut prev_key = start;
		let mut next_key;
		path.push(start);
		while prev_key != destination {
			next_key = match prev_key {
				Self::Unallocated => Self::Id(max_id),
				Self::Id(prev_id) => match prev_key.cmp(&destination) {
					Ordering::Less => {
						if prev_id < max_id {
							Self::Id(prev_id + 1)
						} else {
							Self::Unallocated
						}
					}
					Ordering::Greater => Self::Id(prev_id - 1),
					Ordering::Equal => unimplemented!(),
				},
			};
			path.push(next_key);
			prev_key = next_key;
		}
		Some(path)
	}
}

#[cfg(test)]
mod category_key {
	use super::*;

	#[test]
	fn new_path_unallocated_to_unallocated() {
		let start = CategoryKey::Unallocated;
		let destination = CategoryKey::Unallocated;
		assert_eq!(CategoryKey::new_path(start, destination, 0), None);
	}

	#[test]
	fn new_path_insert() {
		let start = CategoryKey::Unallocated;
		let destination = CategoryKey::Id(3);
		let max_id = 5;
		assert_eq!(
			CategoryKey::new_path(start, destination, max_id),
			Some(vec![
				CategoryKey::Unallocated,
				CategoryKey::Id(5),
				CategoryKey::Id(4),
				CategoryKey::Id(3),
			])
		);
	}

	#[test]
	fn new_path_remove() {
		let start = CategoryKey::Id(2);
		let destination = CategoryKey::Unallocated;
		let max_id = 6;
		assert_eq!(
			CategoryKey::new_path(start, destination, max_id),
			Some(vec![
				CategoryKey::Id(2),
				CategoryKey::Id(3),
				CategoryKey::Id(4),
				CategoryKey::Id(5),
				CategoryKey::Id(6),
				CategoryKey::Unallocated,
			])
		);
	}
}

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
	changed_indices: HashSet<usize>,
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
			changed_indices: HashSet::new(),
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
	pub fn take_changed_indices(&mut self) -> Option<(Vec<std::ops::Range<usize>>, usize)> {
		if self.changed_indices.is_empty() {
			return None;
		}
		profiling::scope!("take_changed_indices");
		let mut indices = self.changed_indices.drain().collect::<Vec<_>>();
		indices.sort();

		let total_count = indices.len();
		let mut ranges = Vec::new();
		let mut range: Option<std::ops::Range<usize>> = None;
		for i in indices.into_iter() {
			if let Some(range) = &mut range {
				if i == range.end {
					range.end += 1;
					continue;
				}
			}
			if let Some(range) = range {
				ranges.push(range);
			}
			range = Some(std::ops::Range {
				start: i,
				end: i + 1,
			});
		}
		if let Some(range) = range {
			ranges.push(range);
		}
		Some((ranges, total_count))
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

	fn get_category_idx(&self, key: CategoryKey) -> usize {
		match key {
			CategoryKey::Id(block_id) => block_id,
			CategoryKey::Unallocated => self.block_type_count,
		}
	}

	fn get_category(&self, key: CategoryKey) -> &Category {
		let idx = self.get_category_idx(key);
		&self.categories[idx]
	}

	fn get_category_mut(&mut self, key: CategoryKey) -> &mut Category {
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
		self.change_category(&point, CategoryKey::Id(prev_id), CategoryKey::Id(next_id));
	}

	/// Deallocates the instance data and removes all reference to the point from the metadata (active AND inactive).
	fn remove(&mut self, point: &block::Point, prev_id: block::LookupId) {
		if let Some(idx) =
			self.change_category(&point, CategoryKey::Id(prev_id), CategoryKey::Unallocated)
		{
			self.instances[idx] = Instance::default();
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
		start: CategoryKey,
		destination: CategoryKey,
	) -> Option<usize> {
		use std::cmp::Ordering;
		let path = match CategoryKey::new_path(start, destination, self.max_block_id()) {
			Some(path) => path,
			None => return None,
		};
		let direction = destination.cmp(&start);
		let mut instance_idx = match start {
			CategoryKey::Unallocated => self.get_category(start).start(),
			_ => match self.active_points.get_mut(&point.chunk()) {
				Some(chunk_points) => match chunk_points.remove(&point.offset()) {
					Some((_id, instance_idx)) => instance_idx,
					None => return None,
				},
				None => return None,
			},
		};
		let mut prev_key = start;
		for key in path.into_iter() {
			// The first item in the path is always the starting category.
			if key == start {
				assert!(self.get_category(key).count() > 0);
				match direction {
					// The destination is "less than"/"to the left of" the start category,
					// so first we move the instance to the start of our current category.
					Ordering::Less => {
						let mut target_idx = self.get_category(key).start();
						self.swap_instances(&mut instance_idx, &mut target_idx);
					}
					// The destination is "more than"/"to the right of" the start category,
					// so first we move the instance to the end of our current category.
					Ordering::Greater => {
						let mut target_idx = self.get_category(key).last();
						self.swap_instances(&mut instance_idx, &mut target_idx);
					}
					_ => unimplemented!(),
				}
			}
			// We are somewhere in the middle of the path
			else if key != destination {
				// The item is either at the start or end of the previous category. We should shrink the previous
				// category and expand our category such that the item at `instance_idx` is now in the next category over.
				match direction {
					Ordering::Less => {
						// Move the item into the current category (its index in the instance list does not change).
						self.get_category_mut(prev_key).shrink_right();
						self.get_category_mut(key).expand_right();
						// Now swap it to the start of the category because we have at least 1 more transition ahead
						let mut target_idx = self.get_category(key).start();
						self.swap_instances(&mut instance_idx, &mut target_idx);
					}
					Ordering::Greater => {
						// Move the item into the current category (its index in the instance list does not change).
						self.get_category_mut(prev_key).shrink_left();
						self.get_category_mut(key).expand_left();
						// Now swap it to the start of the category because we have at least 1 more transition ahead
						let mut target_idx = self.get_category(key).last();
						self.swap_instances(&mut instance_idx, &mut target_idx);
					}
					_ => unimplemented!(),
				}
			}
			// The last item in the path is always the destination category
			else {
				match direction {
					Ordering::Less => {
						// Move the item into the current category (its index in the instance list does not change).
						self.get_category_mut(prev_key).shrink_right();
						self.get_category_mut(key).expand_right();
					}
					Ordering::Greater => {
						// Move the item into the current category (its index in the instance list does not change).
						self.get_category_mut(prev_key).shrink_left();
						self.get_category_mut(key).expand_left();
					}
					_ => unimplemented!(),
				}
			}
			prev_key = key;
		}

		if let CategoryKey::Id(block_id) = destination {
			if let Some(chunk_points) = self.active_points.get_mut(&point.chunk()) {
				let _ = chunk_points.insert(*point.offset(), (block_id, instance_idx));
			}
		}

		Some(instance_idx)
	}

	fn swap_instances(&mut self, a: &mut usize, b: &mut usize) {
		if *a == *b {
			return;
		}
		// Swap the instance data
		self.instances.swap(*a, *b);
		self.changed_indices.insert(*a);
		self.changed_indices.insert(*b);
		// Swap the actual indices provided
		std::mem::swap(a, b);
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

		let all_faces = EnumSet::<Face>::all();
		// For each point in the set, check all of its faces (and update its neighbor if necessary)
		for &primary_point in points.iter() {
			profiling::scope!("update-faces", &format!("{}", primary_point));

			// Get the category for this primary point
			let primary_point_id = self.get_block_id(&primary_point);

			// Gather the block-type of each voxel on a given face of the point
			let mut face_ids = Vec::with_capacity(all_faces.len());
			for primary_point_face in all_faces.iter() {
				profiling::scope!("face", &format!("{}", primary_point_face));
				// Get the block::Point of the block on that face of the primary point
				let secondary_point = primary_point + primary_point_face.direction();
				// And get the block-type of that adjacent point
				let secondary_point_id = self.get_block_id(&secondary_point);
				// Save off the adjacent block information
				face_ids.push((primary_point_face, secondary_point, secondary_point_id));
				// The secondary point could be empty (air). If it is, then it doesnt have a block-id.
				if let Some(secondary_point_id) = secondary_point_id {
					// If the adjacent point is not a primary point, the face that
					// is adjacent to the primary point should also be updated.
					// If it IS a primary point, it has either already been
					// visited or will be visited shortly.
					if !points.contains(&secondary_point) {
						let secondary_point_face = primary_point_face.inverse();
						self.recalculate_faces(
							secondary_point,
							secondary_point_id,
							vec![(secondary_point_face, primary_point, primary_point_id)],
							&model_cache,
						);
					}
				}
			}
			// Update the faces for this primary point
			if let Some(primary_point_id) = primary_point_id {
				self.recalculate_faces(primary_point, primary_point_id, face_ids, &model_cache);
			}
		}
	}

	fn recalculate_faces(
		&mut self,
		point: block::Point,
		id: (IdPhase, block::LookupId),
		faces: Vec<(Face, block::Point, Option<(IdPhase, block::LookupId)>)>,
		model_cache: &Arc<model::Cache>,
	) {
		profiling::scope!(
			"recalculate_faces",
			&format!("point:{} face-count:{}", point, faces.len())
		);

		let mut desired_phase = id.0;
		if let Some((idx, instance)) = self.get_instance_mut(&point, id.0) {
			let mut point_faces = instance.faces();
			for (face, _adj_point, block_id) in faces.into_iter() {
				let face_is_enabled = match block_id {
					// Block doesnt exist at this point (its air/empty) or the chunk isn't loaded.
					None => true,
					Some((_phase, id)) => match model_cache.get(&id) {
						// Found a model, can base face visibility based on if the model is fully-opaque
						Some((model, _, _)) => !model.is_opaque(),
						// No model matches the id... x_x
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
				instance.set_faces(point_faces);
				if let Some(idx) = idx {
					self.changed_indices.insert(idx);
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

		// If it is currently active and no longer has faces to render or vice versa,
		// we need to move it to the correct phase.
		if desired_phase != id.0 {
			self.change_phase(&point, id.0, desired_phase);
		}
	}

	fn change_phase(&mut self, point: &block::Point, prev: IdPhase, next: IdPhase) {
		profiling::scope!("change_phase", &format!("{} {:?}->{:?}", point, prev, next));
		match (prev, next) {
			// Deactivating a block, time to remove it from the buffered data.
			(IdPhase::Active, IdPhase::Inactive) => {
				let (id, instance_idx) = match self.active_points.get_mut(&point.chunk()) {
					Some(chunk_points) => match chunk_points.remove(&point.offset()) {
						Some((id, instance_idx)) => (id, instance_idx),
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
					.change_category(&point, CategoryKey::Unallocated, CategoryKey::Id(id))
					.unwrap();
				match self.instances.get_mut(instance_idx) {
					Some(target) => {
						*target = instance;
					}
					None => return,
				}
			}
			_ => unimplemented!(),
		}
	}
}
