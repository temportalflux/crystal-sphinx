use crate::block;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
	/// A category for a specific block-type.
	Id(block::LookupId),
	/// A unique category full of garbage data.
	/// Holds the segment from which new voxel instances are allocated from.
	Unallocated,
}

impl Key {
	/// Determines the order of segments that need to be traversed in order to move an instance from `start` to `destination`.
	/// If `start == destination`, the returned path will be None, otherwise the resulting path
	/// will always contain `start` as the first value and `destingation` as the last value.
	pub fn new_path(start: Self, destination: Self, max_id: block::LookupId) -> Option<Vec<Self>> {
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
	use super::Key as CategoryKey;

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
