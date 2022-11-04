use super::{Direction, RangeSet, Section, TargetPosition};
use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
};

#[derive(Clone, PartialEq, Eq)]
struct OrderedHashMap<K: Eq + Hash + Clone, V> {
	idx_map: HashMap<K, usize>,
	values: Vec<(K, V)>,
}

impl<K, V> std::fmt::Debug for OrderedHashMap<K, V>
where
	K: Hash + Eq + Clone + std::fmt::Debug + std::cmp::Ord,
	V: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut ids = self.idx_map.iter().collect::<Vec<_>>();
		ids.sort();
		write!(f, "OrderedMap(ids={:?}, values={:?})", ids, self.values)
	}
}

impl<K, V> OrderedHashMap<K, V>
where
	K: Eq + Hash + Clone,
{
	fn new() -> Self {
		Self {
			idx_map: HashMap::new(),
			values: Vec::new(),
		}
	}

	fn len(&self) -> usize {
		self.values.len()
	}

	fn last(&self) -> Option<&V> {
		self.values[..].last().map(|(_, v)| v)
	}

	fn push(&mut self, key: K, value: V) -> usize {
		let idx = self.values.len();
		self.values.push((key.clone(), value));
		self.idx_map.insert(key, idx);
		idx
	}

	fn idx(&self, key: &K) -> Option<usize> {
		self.idx_map.get(key).cloned()
	}

	fn key(&self, idx: usize) -> &K {
		self.values.get(idx).map(|(k, _)| k).unwrap()
	}

	fn value(&self, idx: usize) -> &V {
		self.values.get(idx).map(|(_, v)| v).unwrap()
	}

	fn value_mut(&mut self, idx: usize) -> &mut V {
		self.values.get_mut(idx).map(|(_, v)| v).unwrap()
	}

	fn retain<F>(&mut self, mut should_keep: F)
	where
		F: FnMut(&V) -> bool,
	{
		let mut idx = 0;
		while idx < self.values.len() {
			if should_keep(&self.values[idx].1) {
				*self.idx_map.get_mut(&self.values[idx].0).unwrap() = idx;
				idx += 1;
			} else {
				let (key, _) = self.values.remove(idx);
				self.idx_map.remove(&key);
			}
		}
	}

	fn as_unordered(&self) -> HashMap<&K, &V> {
		self.idx_map
			.iter()
			.map(|(key, idx)| (key, self.value(*idx)))
			.collect()
	}
}

/// A linear vec, sorted into subsections by key association.
/// Operates similar to a MultiMap, except the values are kept in a single vec.
/// Operations are optimized for minimal changes to the underlying vec (which is especially useful for copying to GPU for graphics buffers).
/// Unused/unfilled entries are kept at the end of the vec.
///
/// S: Type of the section identifier
/// K: Type of the value identifier (for reference in operations to refer to the value)
/// V: Type of the actual values
#[derive(Clone, PartialEq, Eq)]
pub struct VecSectioned<S: Eq + Hash + Clone, K: Eq + Hash, V> {
	sections: OrderedHashMap<S, Section>,

	value_key_to_idx: HashMap<K, (Option<S>, usize)>,
	value_idx_to_key: HashMap<usize, K>,
	values: Vec<V>,

	/// Values which have been mutated since the last `take_changed_ranges`.
	changed_ranges: RangeSet,
}

impl<S, K, V> std::fmt::Debug for VecSectioned<S, K, V>
where
	S: Hash + Eq + Clone + std::fmt::Debug + std::cmp::Ord,
	K: Hash + Eq + std::fmt::Debug + std::cmp::Ord,
	V: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut key_map = self.value_key_to_idx.iter().collect::<Vec<_>>();
		key_map.sort();
		let mut idx_map = self.value_idx_to_key.iter().collect::<Vec<_>>();
		idx_map.sort();
		write!(f, "VecSectioned(sections={:?}, values={{key_map={:?}, idx_map={:?}, items={:?}}}) changed_indices={:?}", self.sections, key_map, idx_map, self.values, self.changed_ranges)
	}
}

impl<S, K, V> VecSectioned<S, K, V>
where
	S: Hash + Eq + Clone,
	K: Hash + Eq,
{
	pub fn new() -> Self {
		Self {
			sections: OrderedHashMap::new(),

			value_key_to_idx: HashMap::new(),
			value_idx_to_key: HashMap::new(),
			values: Vec::new(),

			changed_ranges: RangeSet::default(),
		}
	}

	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			sections: OrderedHashMap::new(),

			value_key_to_idx: HashMap::with_capacity(capacity),
			value_idx_to_key: HashMap::with_capacity(capacity),
			values: Vec::with_capacity(capacity),

			changed_ranges: RangeSet::default(),
		}
	}

	/// Returns the length of the value vec, including all idle/unused values.
	pub fn len(&self) -> usize {
		self.values.len()
	}

	/// Returns a set of ranges which were mutated in the values vec.
	/// Also prunes any empty sections from the list.
	#[profiling::function]
	pub fn take_changed_ranges(&mut self) -> Option<(Vec<std::ops::Range<usize>>, usize)> {
		self.sections.retain(|v| !v.is_empty());
		(!self.changed_ranges.is_empty()).then(|| self.changed_ranges.take())
	}

	pub fn sections(&self) -> HashMap<S, std::ops::Range<usize>> {
		let sections = self.sections.as_unordered();
		sections
			.into_iter()
			.map(|(key, section)| (key.clone(), section.inner().clone()))
			.collect()
	}

	pub fn values(&self) -> &Vec<V> {
		&self.values
	}

	#[profiling::function]
	pub fn insert(&mut self, section_id: &S, key: &K, value: V)
	where
		K: Clone,
		S: Clone,
	{
		let section_idx = self.get_or_insert_section(section_id);

		let value_idx = self.values.len();
		self.value_key_to_idx.insert(key.clone(), (None, value_idx));
		self.value_idx_to_key.insert(value_idx, key.clone());
		self.values.push(value);
		self.changed_ranges.insert(value_idx);

		self.change_section(None, Some(section_idx), &key);
	}

	#[profiling::function]
	pub fn update(&mut self, key: &K, value: V) -> bool {
		let Some((_, value_idx)) = self.value_key_to_idx.get(key) else {
			return false;
		};
		let Some(old_value) = self.values.get_mut(*value_idx) else {
			return false;
		};
		*old_value = value;
		self.changed_ranges.insert(*value_idx);
		true
	}

	#[profiling::function]
	pub fn remove(&mut self, key: &K) -> Option<(S, V)>
	where
		S: Clone,
		K: Clone,
	{
		let section_id = match self.get_value_section(key) {
			Some(section_id) => section_id,
			None => return None,
		};
		let section_idx = self.sections.idx(&section_id);
		let value = self.change_section(section_idx, None, key);
		value.map(|v| (section_id, v))
	}

	#[profiling::function]
	pub fn swap(&mut self, key: &K, new_section: &S)
	where
		S: Clone,
		K: Clone,
	{
		let src_section_idx = match self.get_value_section(key) {
			Some(section_id) => self.sections.idx(&section_id),
			None => return,
		};
		let dst_section_idx = Some(self.get_or_insert_section(new_section));
		self.change_section(src_section_idx, dst_section_idx, key);
	}

	fn get_or_insert_section(&mut self, id: &S) -> usize
	where
		S: Clone,
	{
		match self.sections.idx(id) {
			Some(idx) => idx,
			None => self.sections.push(
				id.clone(),
				match self.sections.last() {
					Some(prev) => Section::new(prev.end(), 0),
					None => Section::default(),
				},
			),
		}
	}

	fn get_value_section(&self, key: &K) -> Option<S>
	where
		S: Clone,
	{
		match self.value_key_to_idx.get(key) {
			Some((section_id, _)) => Some(section_id.clone().unwrap()),
			None => None,
		}
	}

	fn value_index_in_section(
		&self,
		section_idx: Option<usize>,
		position: TargetPosition,
	) -> usize {
		match section_idx {
			// Get the position in the section.
			Some(section_idx) => self.sections.value(section_idx).index_at_position(position),
			// The "not in a section" always has 1 item and its at the end of the vec.
			None => self.len() - 1,
		}
	}

	fn value_key_meta(&mut self, value_idx: &usize) -> &mut (Option<S>, usize) {
		self.value_key_to_idx
			.get_mut(self.value_idx_to_key.get(value_idx).unwrap())
			.unwrap()
	}

	/// Moves the value at `key` from the section at the index of `src` to the section at index of `dst`.
	/// If `src` is None, the value at `key` is moved into `dst` from the end of the `values` vec.
	/// If `dst` is None, the value at `key` is moved from `src` to the end of the `values` vec,
	/// and then removed from the vec and returned (the key value in `value_hash` is also removed during this operation).
	/// Any changes to the underlying `values` vec is marked in the `changed_ranges` property, and section boundaries are updated as well.
	fn change_section(&mut self, src: Option<usize>, dst: Option<usize>, key: &K) -> Option<V>
	where
		S: Clone,
		K: Clone,
	{
		let (direction, touched_sections) = match Self::make_idx_path(src, dst, self.sections.len())
		{
			Some((direction, sections)) => (direction, sections),
			None => return None,
		};
		let mut prev_value_idx = self.value_key_to_idx.get(key).map(|(_, idx)| *idx).unwrap();

		let mut prev_section = src;
		let mut affected_value_indices = HashSet::new();
		for next_section in touched_sections.into_iter() {
			// 1st section in path (prev_section = next_section = src): move the value to the correct slot in the src section.
			// As such, this operation isn't relevant if next_section == src.
			if next_section != src {
				// Apply section index shifts/mutations to interpret the value to be under the next section.
				let (operations, position_newly_in_next) =
					direction.operations(prev_section, next_section);
				for (section_idx, operation) in operations.into_iter() {
					if let Some(idx) = section_idx {
						let section = self.sections.value_mut(idx);
						section.apply(operation);
					}
				}
				// Update the value whose section has changed as a result of the operations.
				{
					let value_idx =
						self.value_index_in_section(next_section, position_newly_in_next);
					self.value_key_meta(&value_idx).0 =
						next_section.map(|sidx| self.sections.key(sidx)).cloned();
				}
			}
			// 1st section in path: Shift the value to the first or last slot in that section (so it can be moved in subsequent operations).
			// Last section in path: the above operations have interpretted the current position of the
			// value to be in the destination, so no more changes are required.
			if next_section != dst {
				// Since this is not the destination section, the only time next_section is None is when its the starting section (so next_section = src).
				// This will always mean that the value in question is the last one in `values`, and is the only one not in a section.
				// So moving it within the section is a no-op, its already prepared to be shuffled.
				if let Some(next_section_idx) = next_section {
					let next_value_idx = {
						let section = self.sections.value(next_section_idx);
						section.index_at_position(direction.target_position())
					};
					// If the values were actually swapped, then we should mark the indices as changed.
					if self.swap_values(prev_value_idx, next_value_idx) {
						if affected_value_indices.len() == 0 {
							affected_value_indices.insert(prev_value_idx);
						}
						affected_value_indices.insert(next_value_idx);

						// The value we are moving is now at next_value_idx.
						prev_value_idx = next_value_idx;
					}
				}
			}

			prev_section = next_section;
		}
		// Append all changed indices to the changed index ranges.
		for value_idx in affected_value_indices.into_iter() {
			self.changed_ranges.insert(value_idx);
		}

		// If the destination is remove-from-sections, remove the value and return it.
		let removed_value = match dst.is_some() {
			true => None,
			false => {
				let value = self.values.remove(prev_value_idx);
				self.value_key_to_idx.remove(key);
				self.value_idx_to_key.remove(&prev_value_idx);
				Some(value)
			}
		};

		removed_value
	}

	fn make_idx_path(
		src: Option<usize>,
		dst: Option<usize>,
		len: usize,
	) -> Option<(Direction, Vec<Option<usize>>)> {
		if src == dst {
			return None;
		}

		let mut touched_sections = Vec::new();
		let mut push_section = |idx| {
			touched_sections.push(match idx {
				// If the idx is a valid index in the range
				idx if idx < len => Some(idx),
				// Otherwise if its out of bounds, then its none
				_ => None,
			});
		};

		let mut current = src.unwrap_or(len);
		let dst_num = dst.unwrap_or(len);
		let direction = Direction::from(&current, &dst_num);

		while current != dst_num {
			push_section(current);
			current += direction;
		}
		// Current is now dst, and the path should include the final destination.
		push_section(current);

		Some((direction, touched_sections))
	}

	fn swap_values(&mut self, a_idx: usize, b_idx: usize) -> bool
	where
		S: Clone,
		K: Clone,
	{
		if a_idx == b_idx {
			return false;
		}

		self.values.swap(a_idx, b_idx);

		// Update metadata about the two items which were just swapped
		let a_key = self.value_idx_to_key.get(&a_idx).unwrap().clone();
		let a_section = self.value_key_to_idx.get_mut(&a_key).unwrap().0.take();
		let b_key = self.value_idx_to_key.get(&b_idx).unwrap().clone();
		let b_section = self.value_key_to_idx.get_mut(&b_key).unwrap().0.take();

		// value prev at b is now at a
		self.value_key_to_idx
			.insert(b_key.clone(), (a_section, a_idx));
		self.value_idx_to_key.insert(a_idx, b_key);

		// value prev at a is now at b
		self.value_key_to_idx
			.insert(a_key.clone(), (b_section, b_idx));
		self.value_idx_to_key.insert(b_idx, a_key);

		true
	}
}

/// These tests intentionally check the data in the structures instead of checking the puiblic-interface for the structure.
/// VecSectioned is particularly complex, so when writing these tests the intent was to make sure
/// the data being changed in the exact way I expect it to be.
/// There may be a world in which additional tests get added to just check the public-interface.
/// Having two separate sets of tests for this would make it clear when a tests fails, is it because the data is malinged
/// or because the data being returned from the public-interface is incorrect.
#[cfg(test)]
mod test {
	use super::*;

	#[cfg(test)]
	mod make_idx_path {
		use super::*;

		#[test]
		fn no_change() {
			let path = VecSectioned::<char, u32, f32>::make_idx_path(Some(3), Some(3), 6);
			assert_eq!(path, None);
		}

		#[test]
		fn right_existing() {
			let path = VecSectioned::<char, u32, f32>::make_idx_path(Some(1), Some(4), 6);
			let expected = vec![Some(1), Some(2), Some(3), Some(4)];
			assert_eq!(path, Some((Direction::Right, expected)));
		}

		#[test]
		fn left_existing() {
			let path = VecSectioned::<char, u32, f32>::make_idx_path(Some(3), Some(1), 6);
			let expected = vec![Some(3), Some(2), Some(1)];
			assert_eq!(path, Some((Direction::Left, expected)));
		}

		#[test]
		fn remove() {
			let path = VecSectioned::<char, u32, f32>::make_idx_path(Some(2), None, 6);
			let expected = vec![Some(2), Some(3), Some(4), Some(5), None];
			assert_eq!(path, Some((Direction::Right, expected)));
		}

		#[test]
		fn insert() {
			let path = VecSectioned::<char, u32, f32>::make_idx_path(None, Some(4), 6);
			let expected = vec![None, Some(5), Some(4)];
			assert_eq!(path, Some((Direction::Left, expected)));
		}
	}

	#[cfg(test)]
	mod swap_values {
		use super::*;

		#[test]
		fn swap_same_section() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0)]),
					values: vec![("section1", Section::from(0..2))],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section1"), 1)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2")]),
				values: vec![0, 1],
				changed_ranges: RangeSet::default(),
			};
			let expected = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0)]),
					values: vec![("section1", Section::from(0..2))],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 1)),
					("item2", (Some("section1"), 0)),
				]),
				value_idx_to_key: HashMap::from([(1, "item1"), (0, "item2")]),
				values: vec![1, 0],
				changed_ranges: RangeSet::default(),
			};
			sectioned.swap_values(0, 1);
			assert_eq!(sectioned, expected);
		}

		#[test]
		fn swap_in_place() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0)]),
					values: vec![("section1", Section::from(0..2))],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section1"), 1)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2")]),
				values: vec![0, 1],
				changed_ranges: RangeSet::default(),
			};
			let expected = sectioned.clone();
			sectioned.swap_values(1, 1);
			assert_eq!(sectioned, expected);
		}

		#[test]
		fn swap_sections() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..5)),
						("section3", Section::from(5..6)),
						("section4", Section::from(6..9)),
						("section5", Section::from(9..12)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item01", (Some("section1"), 0)),
					("item02", (Some("section1"), 1)),
					("item03", (Some("section2"), 2)),
					("item04", (Some("section2"), 3)),
					("item05", (Some("section2"), 4)),
					("item06", (Some("section3"), 5)),
					("item07", (Some("section4"), 6)),
					("item08", (Some("section4"), 7)),
					("item09", (Some("section4"), 8)),
					("item10", (Some("section5"), 9)),
					("item11", (Some("section5"), 10)),
					("item12", (Some("section5"), 11)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "item01"),
					(1, "item02"),
					(2, "item03"),
					(3, "item04"),
					(4, "item05"),
					(5, "item06"),
					(6, "item07"),
					(7, "item08"),
					(8, "item09"),
					(9, "item10"),
					(10, "item11"),
					(11, "item12"),
				]),
				values: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
				changed_ranges: RangeSet::default(),
			};
			let expected = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..5)),
						("section3", Section::from(5..6)),
						("section4", Section::from(6..9)),
						("section5", Section::from(9..12)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item01", (Some("section1"), 0)),
					("item03", (Some("section1"), 1)),
					("item11", (Some("section2"), 2)),
					("item04", (Some("section2"), 3)),
					("item06", (Some("section2"), 4)),
					("item05", (Some("section3"), 5)),
					("item08", (Some("section4"), 6)),
					("item07", (Some("section4"), 7)),
					("item10", (Some("section4"), 8)),
					("item09", (Some("section5"), 9)),
					("item02", (Some("section5"), 10)),
					("item12", (Some("section5"), 11)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "item01"),
					(1, "item03"),
					(2, "item11"),
					(3, "item04"),
					(4, "item06"),
					(5, "item05"),
					(6, "item08"),
					(7, "item07"),
					(8, "item10"),
					(9, "item09"),
					(10, "item02"),
					(11, "item12"),
				]),
				values: vec![0, 2, 10, 3, 5, 4, 7, 6, 9, 8, 1, 11],
				changed_ranges: RangeSet::default(),
			};
			sectioned.swap_values(1, 2);
			sectioned.swap_values(6, 7);
			sectioned.swap_values(9, 8);
			sectioned.swap_values(4, 5);
			sectioned.swap_values(2, 10);
			assert_eq!(sectioned, expected);
		}

		#[test]
		fn swap_many() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0), ("section2", 1)]),
					values: vec![
						("section1", Section::from(0..1)),
						("section2", Section::from(1..2)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section2"), 1)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2")]),
				values: vec![0, 1],
				changed_ranges: RangeSet::default(),
			};
			let expected = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0), ("section2", 1)]),
					values: vec![
						("section1", Section::from(0..1)),
						("section2", Section::from(1..2)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section2"), 1)),
					("item2", (Some("section1"), 0)),
				]),
				value_idx_to_key: HashMap::from([(1, "item1"), (0, "item2")]),
				values: vec![1, 0],
				changed_ranges: RangeSet::default(),
			};
			sectioned.swap_values(0, 1);
			assert_eq!(sectioned, expected);
		}
	}

	#[cfg(test)]
	mod take_changed {
		use super::*;

		#[test]
		fn no_changes() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0)]),
					values: vec![("section1", Section::from(0..3))],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section1"), 1)),
					("item3", (Some("section1"), 2)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2"), (2, "item3")]),
				values: vec![0, 1, 2],
				changed_ranges: RangeSet::default(),
			};
			assert_eq!(sectioned.take_changed_ranges(), None);
		}

		#[test]
		fn no_empty_sections() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0), ("section2", 1), ("section3", 2)]),
					values: vec![
						("section1", Section::from(0..1)),
						("section2", Section::from(1..2)),
						("section3", Section::from(2..3)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section2"), 1)),
					("item3", (Some("section3"), 2)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2"), (2, "item3")]),
				values: vec![0, 1, 2],
				changed_ranges: RangeSet {
					ranges: vec![0..1],
					total_size: 1,
				},
			};
			assert_eq!(sectioned.take_changed_ranges(), Some((vec![0..1], 1)));

			sectioned.changed_ranges = RangeSet {
				ranges: vec![0..1, 4..5, 6..10],
				total_size: 6,
			};
			assert_eq!(
				sectioned.take_changed_ranges(),
				Some((vec![0..1, 4..5, 6..10], 6))
			);
		}

		#[test]
		fn full_prune() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..2)),
						("section3", Section::from(2..6)),
						("section4", Section::from(6..6)),
						("section5", Section::from(6..9)),
					],
				},
				value_key_to_idx: HashMap::from([
					("itemA", (Some("section1"), 0)),
					("itemB", (Some("section1"), 1)),
					("itemF", (Some("section3"), 2)),
					("itemD", (Some("section3"), 3)),
					("itemE", (Some("section3"), 4)),
					("itemI", (Some("section3"), 5)),
					("itemG", (Some("section5"), 6)),
					("itemH", (Some("section5"), 7)),
					("itemJ", (Some("section5"), 8)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "itemA"),
					(1, "itemB"),
					(2, "itemF"),
					(3, "itemD"),
					(4, "itemE"),
					(5, "itemI"),
					(6, "itemG"),
					(7, "itemH"),
					(8, "itemJ"),
				]),
				values: vec!['a', 'b', 'f', 'd', 'e', 'i', 'g', 'h', 'j'],

				changed_ranges: RangeSet {
					ranges: vec![2..3, 5..6, 8..10],
					total_size: 4,
				},
			};
			assert_eq!(
				sectioned.take_changed_ranges(),
				Some((vec![2..3, 5..6, 8..10], 4))
			);
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from(
							[("section1", 0), ("section3", 1), ("section5", 2),]
						),
						values: vec![
							("section1", Section::from(0..2)),
							("section3", Section::from(2..6)),
							("section5", Section::from(6..9)),
						],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
						("itemF", (Some("section3"), 2)),
						("itemD", (Some("section3"), 3)),
						("itemE", (Some("section3"), 4)),
						("itemI", (Some("section3"), 5)),
						("itemG", (Some("section5"), 6)),
						("itemH", (Some("section5"), 7)),
						("itemJ", (Some("section5"), 8)),
					]),
					value_idx_to_key: HashMap::from([
						(0, "itemA"),
						(1, "itemB"),
						(2, "itemF"),
						(3, "itemD"),
						(4, "itemE"),
						(5, "itemI"),
						(6, "itemG"),
						(7, "itemH"),
						(8, "itemJ"),
					]),
					values: vec!['a', 'b', 'f', 'd', 'e', 'i', 'g', 'h', 'j'],
					changed_ranges: RangeSet::default(),
				}
			);
		}
	}

	#[cfg(test)]
	mod change_section {
		use super::*;

		#[test]
		fn same_section() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0)]),
					values: vec![("section1", Section::from(0..3))],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section1"), 1)),
					("item3", (Some("section1"), 2)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2"), (2, "item3")]),
				values: vec![0, 1, 2],
				changed_ranges: RangeSet::default(),
			};
			let expected = sectioned.clone();
			sectioned.change_section(Some(0), Some(0), &"item2");
			assert_eq!(sectioned, expected);
		}

		#[test]
		fn two_sections_one_item_each() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0), ("section2", 1)]),
					values: vec![
						("section1", Section::from(0..1)),
						("section2", Section::from(1..3)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section2"), 1)),
					("item3", (Some("section2"), 2)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2"), (2, "item3")]),
				values: vec![0, 1, 2],
				changed_ranges: RangeSet::default(),
			};
			let expected = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([("section1", 0), ("section2", 1)]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..3)),
					],
				},
				value_key_to_idx: HashMap::from([
					("item1", (Some("section1"), 0)),
					("item2", (Some("section1"), 1)),
					("item3", (Some("section2"), 2)),
				]),
				value_idx_to_key: HashMap::from([(0, "item1"), (1, "item2"), (2, "item3")]),
				values: vec![0, 1, 2],
				changed_ranges: RangeSet::default(),
			};

			let path =
				VecSectioned::<&str, &str, i32>::make_idx_path(Some(1), Some(0), sectioned.len());
			assert_eq!(path, Some((Direction::Left, vec![Some(1), Some(0)])));

			let removed = sectioned.change_section(Some(1), Some(0), &"item2");
			assert_eq!(removed, None);

			assert_eq!(sectioned, expected);
		}

		#[test]
		fn many_total_length_maintained() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..1)),
						("section2", Section::from(1..3)),
						("section3", Section::from(3..6)),
						("section4", Section::from(6..9)),
						("section5", Section::from(9..10)),
					],
				},
				value_key_to_idx: HashMap::from([
					("itemA", (Some("section1"), 0)),
					("itemB", (Some("section2"), 1)),
					("itemC", (Some("section2"), 2)),
					("itemD", (Some("section3"), 3)),
					("itemE", (Some("section3"), 4)),
					("itemF", (Some("section3"), 5)),
					("itemG", (Some("section4"), 6)),
					("itemH", (Some("section4"), 7)),
					("itemI", (Some("section4"), 8)),
					("itemJ", (Some("section5"), 9)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "itemA"),
					(1, "itemB"),
					(2, "itemC"),
					(3, "itemD"),
					(4, "itemE"),
					(5, "itemF"),
					(6, "itemG"),
					(7, "itemH"),
					(8, "itemI"),
					(9, "itemJ"),
				]),
				values: vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
				changed_ranges: RangeSet::default(),
			};

			// No data changes, just change the section id
			let removed = sectioned.change_section(Some(1), Some(0), &"itemB");
			assert_eq!(removed, None);
			let expected_b = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..3)),
						("section3", Section::from(3..6)),
						("section4", Section::from(6..9)),
						("section5", Section::from(9..10)),
					],
				},
				value_key_to_idx: HashMap::from([
					("itemA", (Some("section1"), 0)),
					("itemB", (Some("section1"), 1)),
					("itemC", (Some("section2"), 2)),
					("itemD", (Some("section3"), 3)),
					("itemE", (Some("section3"), 4)),
					("itemF", (Some("section3"), 5)),
					("itemG", (Some("section4"), 6)),
					("itemH", (Some("section4"), 7)),
					("itemI", (Some("section4"), 8)),
					("itemJ", (Some("section5"), 9)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "itemA"),
					(1, "itemB"),
					(2, "itemC"),
					(3, "itemD"),
					(4, "itemE"),
					(5, "itemF"),
					(6, "itemG"),
					(7, "itemH"),
					(8, "itemI"),
					(9, "itemJ"),
				]),
				values: vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
				changed_ranges: RangeSet::default(),
			};
			assert_eq!(sectioned, expected_b);

			// Move between two adjacent categories, affecting a number of indices along the way
			let removed = sectioned.change_section(Some(2), Some(3), &"itemD");
			assert_eq!(removed, None);
			let expected_d = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..3)),
						("section3", Section::from(3..5)),
						("section4", Section::from(5..9)),
						("section5", Section::from(9..10)),
					],
				},
				value_key_to_idx: HashMap::from([
					("itemA", (Some("section1"), 0)),
					("itemB", (Some("section1"), 1)),
					("itemC", (Some("section2"), 2)),
					("itemF", (Some("section3"), 3)),
					("itemE", (Some("section3"), 4)),
					("itemD", (Some("section4"), 5)),
					("itemG", (Some("section4"), 6)),
					("itemH", (Some("section4"), 7)),
					("itemI", (Some("section4"), 8)),
					("itemJ", (Some("section5"), 9)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "itemA"),
					(1, "itemB"),
					(2, "itemC"),
					(3, "itemF"),
					(4, "itemE"),
					(5, "itemD"),
					(6, "itemG"),
					(7, "itemH"),
					(8, "itemI"),
					(9, "itemJ"),
				]),
				values: vec![10, 11, 12, 15, 14, 13, 16, 17, 18, 19],
				changed_ranges: RangeSet {
					ranges: vec![3..4, 5..6],
					total_size: 2,
				},
			};
			assert_eq!(sectioned, expected_d);

			// Move between non-adjacent categories
			let removed = sectioned.change_section(Some(0), Some(4), &"itemA");
			assert_eq!(removed, None);
			let expected_a = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..1)),
						("section2", Section::from(1..2)),
						("section3", Section::from(2..4)),
						("section4", Section::from(4..8)),
						("section5", Section::from(8..10)),
					],
				},
				value_key_to_idx: HashMap::from([
					("itemB", (Some("section1"), 0)),
					("itemC", (Some("section2"), 1)),
					("itemE", (Some("section3"), 2)),
					("itemF", (Some("section3"), 3)),
					("itemI", (Some("section4"), 4)),
					("itemD", (Some("section4"), 5)),
					("itemG", (Some("section4"), 6)),
					("itemH", (Some("section4"), 7)),
					("itemA", (Some("section5"), 8)),
					("itemJ", (Some("section5"), 9)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "itemB"),
					(1, "itemC"),
					(2, "itemE"),
					(3, "itemF"),
					(4, "itemI"),
					(5, "itemD"),
					(6, "itemG"),
					(7, "itemH"),
					(8, "itemA"),
					(9, "itemJ"),
				]),
				values: vec![11, 12, 14, 15, 18, 13, 16, 17, 10, 19],
				changed_ranges: RangeSet {
					ranges: vec![0..6, 8..9],
					total_size: 7,
				},
			};
			assert_eq!(sectioned, expected_a);
		}

		#[test]
		fn many_insert() {
			let mut sectioned = VecSectioned::new();

			sectioned.insert(&"section1", &"itemA", "a");
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([("section1", 0)]),
						values: vec![("section1", Section::from(0..1))],
					},
					value_key_to_idx: HashMap::from([("itemA", (Some("section1"), 0))]),
					value_idx_to_key: HashMap::from([(0, "itemA")]),
					values: vec!["a"],

					changed_ranges: RangeSet {
						ranges: vec![0..1],
						total_size: 1,
					},
				}
			);

			sectioned.insert(&"section1", &"itemB", "b");
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([("section1", 0)]),
						values: vec![("section1", Section::from(0..2))],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
					]),
					value_idx_to_key: HashMap::from([(0, "itemA"), (1, "itemB")]),
					values: vec!["a", "b"],

					changed_ranges: RangeSet {
						ranges: vec![0..2],
						total_size: 2,
					},
				}
			);

			sectioned.insert(&"section2", &"itemC", "c");
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([("section1", 0), ("section2", 1)]),
						values: vec![
							("section1", Section::from(0..2)),
							("section2", Section::from(2..3))
						],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
						("itemC", (Some("section2"), 2)),
					]),
					value_idx_to_key: HashMap::from([(0, "itemA"), (1, "itemB"), (2, "itemC")]),
					values: vec!["a", "b", "c"],

					changed_ranges: RangeSet {
						ranges: vec![0..3],
						total_size: 3,
					},
				}
			);

			sectioned.insert(&"section3", &"itemD", "d");
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([("section1", 0), ("section2", 1), ("section3", 2)]),
						values: vec![
							("section1", Section::from(0..2)),
							("section2", Section::from(2..3)),
							("section3", Section::from(3..4)),
						],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
						("itemC", (Some("section2"), 2)),
						("itemD", (Some("section3"), 3)),
					]),
					value_idx_to_key: HashMap::from([
						(0, "itemA"),
						(1, "itemB"),
						(2, "itemC"),
						(3, "itemD")
					]),
					values: vec!["a", "b", "c", "d"],

					changed_ranges: RangeSet {
						ranges: vec![0..4],
						total_size: 4,
					},
				}
			);

			sectioned.insert(&"section2", &"itemE", "e");
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([("section1", 0), ("section2", 1), ("section3", 2)]),
						values: vec![
							("section1", Section::from(0..2)),
							("section2", Section::from(2..4)),
							("section3", Section::from(4..5)),
						],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
						("itemC", (Some("section2"), 2)),
						("itemE", (Some("section2"), 3)),
						("itemD", (Some("section3"), 4)),
					]),
					value_idx_to_key: HashMap::from([
						(0, "itemA"),
						(1, "itemB"),
						(2, "itemC"),
						(3, "itemE"),
						(4, "itemD"),
					]),
					values: vec!["a", "b", "c", "e", "d"],

					changed_ranges: RangeSet {
						ranges: vec![0..5],
						total_size: 5,
					},
				}
			);

			sectioned.insert(&"section1", &"itemF", "f");
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([("section1", 0), ("section2", 1), ("section3", 2)]),
						values: vec![
							("section1", Section::from(0..3)),
							("section2", Section::from(3..5)),
							("section3", Section::from(5..6)),
						],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
						("itemF", (Some("section1"), 2)),
						("itemE", (Some("section2"), 3)),
						("itemC", (Some("section2"), 4)),
						("itemD", (Some("section3"), 5)),
					]),
					value_idx_to_key: HashMap::from([
						(0, "itemA"),
						(1, "itemB"),
						(2, "itemF"),
						(3, "itemE"),
						(4, "itemC"),
						(5, "itemD"),
					]),
					values: vec!["a", "b", "f", "e", "c", "d"],

					changed_ranges: RangeSet {
						ranges: vec![0..6],
						total_size: 6,
					},
				}
			);
		}

		#[test]
		fn section_removed() {
			let mut sectioned = VecSectioned {
				sections: OrderedHashMap {
					idx_map: HashMap::from([
						("section1", 0),
						("section2", 1),
						("section3", 2),
						("section4", 3),
						("section5", 4),
					]),
					values: vec![
						("section1", Section::from(0..2)),
						("section2", Section::from(2..3)),
						("section3", Section::from(3..6)),
						("section4", Section::from(6..9)),
						("section5", Section::from(9..10)),
					],
				},
				value_key_to_idx: HashMap::from([
					("itemA", (Some("section1"), 0)),
					("itemB", (Some("section1"), 1)),
					("itemC", (Some("section2"), 2)),
					("itemD", (Some("section3"), 3)),
					("itemE", (Some("section3"), 4)),
					("itemF", (Some("section3"), 5)),
					("itemG", (Some("section4"), 6)),
					("itemH", (Some("section4"), 7)),
					("itemI", (Some("section4"), 8)),
					("itemJ", (Some("section5"), 9)),
				]),
				value_idx_to_key: HashMap::from([
					(0, "itemA"),
					(1, "itemB"),
					(2, "itemC"),
					(3, "itemD"),
					(4, "itemE"),
					(5, "itemF"),
					(6, "itemG"),
					(7, "itemH"),
					(8, "itemI"),
					(9, "itemJ"),
				]),
				values: vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
				changed_ranges: RangeSet::default(),
			};

			let removed = sectioned.change_section(Some(1), None, &"itemC");
			assert_eq!(removed, Some('c'));
			assert_eq!(
				sectioned,
				VecSectioned {
					sections: OrderedHashMap {
						idx_map: HashMap::from([
							("section1", 0),
							("section2", 1),
							("section3", 2),
							("section4", 3),
							("section5", 4),
						]),
						values: vec![
							("section1", Section::from(0..2)),
							("section2", Section::from(2..2)),
							("section3", Section::from(2..5)),
							("section4", Section::from(5..8)),
							("section5", Section::from(8..9)),
						],
					},
					value_key_to_idx: HashMap::from([
						("itemA", (Some("section1"), 0)),
						("itemB", (Some("section1"), 1)),
						("itemF", (Some("section3"), 2)),
						("itemD", (Some("section3"), 3)),
						("itemE", (Some("section3"), 4)),
						("itemI", (Some("section4"), 5)),
						("itemG", (Some("section4"), 6)),
						("itemH", (Some("section4"), 7)),
						("itemJ", (Some("section5"), 8)),
					]),
					value_idx_to_key: HashMap::from([
						(0, "itemA"),
						(1, "itemB"),
						(2, "itemF"),
						(3, "itemD"),
						(4, "itemE"),
						(5, "itemI"),
						(6, "itemG"),
						(7, "itemH"),
						(8, "itemJ"),
					]),
					values: vec!['a', 'b', 'f', 'd', 'e', 'i', 'g', 'h', 'j'],

					changed_ranges: RangeSet {
						ranges: vec![2..3, 5..6, 8..10],
						total_size: 4,
					},
				}
			);
		}
	}
}
