use std::cmp::Ordering;
use std::ops::Range;

/// An ordered set of ranges.
/// Used to keep track of what indices have changed in a vec, without having a ginormous HashSet of usize indices.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RangeSet {
	pub(super) ranges: Vec<Range<usize>>,
	pub(super) total_size: usize,
}

impl std::fmt::Debug for RangeSet {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let ranges_fmt = self
			.ranges
			.iter()
			.map(|range| {
				format!(
					"[{},{})#{}",
					range.start,
					range.end,
					range.end - range.start
				)
			})
			.collect::<Vec<_>>();
		write!(
			f,
			"RangeSet(count={:?}, ranges=[{}])",
			self.total_size,
			ranges_fmt.join(", ")
		)
	}
}

impl RangeSet {
	#[profiling::function]
	pub fn insert(&mut self, idx: usize) {
		let possible_range_idx = match self.get_uninserted_range_idx(idx) {
			Some(idx) => idx,
			None => return,
		};
		self.append_at(idx, possible_range_idx);
		self.merge_ranges_around(possible_range_idx);
	}

	fn get_uninserted_range_idx(&self, idx_to_insert: usize) -> Option<usize> {
		// Ok(index) means that a range was found which contains `idx`.
		// Err(index) means that no range currently contains `idx`,
		// but that the range at `index` could be expanded (start -= 1) if `idx` == `start - 1`
		// or range at `index - 1` could be expanded (end += 1) if `idx` == `end`.
		let result = self.ranges.binary_search_by(|range| -> Ordering {
			if range.end <= idx_to_insert {
				return Ordering::Less;
			}
			if idx_to_insert < range.start {
				return Ordering::Greater;
			}
			Ordering::Equal
		});
		match result {
			// Some range contains the index already
			Ok(_range_idx) => None,
			Err(possible_range_idx) => Some(possible_range_idx),
		}
	}

	fn append_at(&mut self, idx_to_insert: usize, range_idx: usize) {
		// The index is new, so the total count increments
		self.total_size += 1;
		// Insert the new range, maintaining sort order
		self.ranges.insert(
			range_idx,
			Range {
				start: idx_to_insert,   // inclusive
				end: idx_to_insert + 1, // exclusive
			},
		);
	}

	pub fn is_empty(&self) -> bool {
		self.ranges.is_empty()
	}

	#[profiling::function]
	pub fn take(&mut self) -> (Vec<Range<usize>>, usize) {
		let ranges = self.ranges.drain(..).collect();
		let total_size = self.total_size;
		self.total_size = 0;
		(ranges, total_size)
	}

	/// Attempts to merge the range at `range_idx` with the one immediate preceeding and succeeding it.
	#[profiling::function]
	fn merge_ranges_around(&mut self, mut range_idx: usize) {
		// Try merge `range_idx - 1` into `range_idx`.
		if range_idx > 0 && self.can_merge(range_idx - 1, range_idx) {
			self.merge(range_idx - 1, range_idx);
			// If we merged, then the item at `range_idx` is now a part of `range_idx - 1`.
			range_idx = range_idx - 1;
		}

		// Try merge `range_idx` into `range_idx + 1`.
		if range_idx + 1 < self.ranges.len() && self.can_merge(range_idx, range_idx + 1) {
			self.merge(range_idx, range_idx + 1);
		}
	}

	fn can_merge(&self, r1_idx: usize, r2_idx: usize) -> bool {
		// Determine if both ranges exist (neither index is out of bounds)
		// and if the ranges are consecutive.
		match (self.ranges.get(r1_idx), self.ranges.get(r2_idx)) {
			(Some(r1), Some(r2)) => {
				// The ranges are consecutive if the end of the first (exclusive)
				// is equal to the start of the second (inclusive).
				r1.end == r2.start
			}
			_ => false,
		}
	}

	fn merge(&mut self, r1_idx: usize, r2_idx: usize) {
		let is_correct_order = self.ranges[r1_idx].start < self.ranges[r2_idx].start;
		let is_contiguous = self.ranges[r1_idx].end == self.ranges[r2_idx].start;
		if !is_correct_order || !is_contiguous {
			return;
		}
		let r2 = self.ranges.remove(r2_idx);
		let r1 = self.ranges.get_mut(r1_idx).unwrap();
		r1.end = r2.end;
	}
}

#[cfg(test)]
mod range_set {
	use super::*;

	#[test]
	fn can_merge_noncontiguous() {
		let range_set = RangeSet {
			ranges: vec![
				Range { start: 0, end: 1 },
				Range { start: 2, end: 4 },
				Range { start: 7, end: 10 },
			],
			total_size: 6,
		};
		assert_eq!(range_set.can_merge(0, 1), false);
		assert_eq!(range_set.can_merge(1, 2), false);
	}

	#[test]
	fn can_merge_contiguous() {
		let range_set = RangeSet {
			ranges: vec![
				Range { start: 0, end: 1 },
				Range { start: 1, end: 4 },
				Range { start: 4, end: 10 },
			],
			total_size: 10,
		};
		assert_eq!(range_set.can_merge(0, 1), true);
		assert_eq!(range_set.can_merge(1, 2), true);
	}

	#[test]
	fn merge_noncontiguous() {
		let expected = RangeSet {
			ranges: vec![
				Range { start: 0, end: 1 },
				Range { start: 2, end: 4 },
				Range { start: 7, end: 10 },
			],
			total_size: 6,
		};
		let mut range_set = expected.clone();
		range_set.merge(1, 2);
		assert_eq!(range_set, expected);
		range_set.merge(0, 1);
		assert_eq!(range_set, expected);
	}

	#[test]
	fn merge_contiguous() {
		let mut range_set = RangeSet {
			ranges: vec![
				Range { start: 0, end: 1 },
				Range { start: 1, end: 4 },
				Range { start: 4, end: 10 },
			],
			total_size: 10,
		};

		let expected1 = RangeSet {
			ranges: vec![Range { start: 0, end: 1 }, Range { start: 1, end: 10 }],
			total_size: 10,
		};
		range_set.merge(1, 2);
		assert_eq!(range_set, expected1);

		let expected2 = RangeSet {
			ranges: vec![Range { start: 0, end: 10 }],
			total_size: 10,
		};
		range_set.merge(0, 1);
		assert_eq!(range_set, expected2);
	}

	#[test]
	fn merge_around() {
		let mut range_set = RangeSet {
			ranges: vec![
				Range { start: 0, end: 1 },
				Range { start: 2, end: 5 },
				Range { start: 5, end: 8 },
				Range { start: 8, end: 9 },
				Range { start: 10, end: 15 },
			],
			total_size: 13,
		};

		let expected = RangeSet {
			ranges: vec![
				Range { start: 0, end: 1 },
				Range { start: 2, end: 9 },
				Range { start: 10, end: 15 },
			],
			total_size: 13,
		};
		range_set.merge_ranges_around(2);
		assert_eq!(range_set, expected);
	}

	#[test]
	fn take() {
		let ranges = vec![
			Range { start: 0, end: 1 },
			Range { start: 2, end: 9 },
			Range { start: 10, end: 15 },
		];
		let mut range_set = RangeSet {
			ranges: ranges.clone(),
			total_size: 13,
		};
		assert_eq!(range_set.take(), (ranges, 13));
	}

	#[test]
	fn is_empty() {
		assert_eq!(RangeSet::default().is_empty(), true);
		let range_set = RangeSet {
			ranges: vec![Range { start: 0, end: 1 }],
			total_size: 1,
		};
		assert_eq!(range_set.is_empty(), false);
	}

	#[test]
	fn append() {
		let mut range_set = RangeSet {
			ranges: Vec::new(),
			total_size: 0,
		};
		range_set.append_at(1, 0);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![Range { start: 1, end: 2 }],
				total_size: 1,
			}
		);
		range_set.append_at(2, 1);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![Range { start: 1, end: 2 }, Range { start: 2, end: 3 }],
				total_size: 2,
			}
		);
		range_set.append_at(5, 2);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![
					Range { start: 1, end: 2 },
					Range { start: 2, end: 3 },
					Range { start: 5, end: 6 }
				],
				total_size: 3,
			}
		);
	}

	#[test]
	fn find_idx_to_insert_empty() {
		let range_set = RangeSet {
			ranges: Vec::new(),
			total_size: 0,
		};
		assert_eq!(range_set.get_uninserted_range_idx(5), Some(0));
	}

	#[test]
	fn find_idx_to_insert_existing() {
		let range_set = RangeSet {
			ranges: vec![
				Range { start: 1, end: 2 },
				Range { start: 2, end: 3 },
				Range { start: 5, end: 6 },
			],
			total_size: 3,
		};
		assert_eq!(range_set.get_uninserted_range_idx(2), None);
		assert_eq!(range_set.get_uninserted_range_idx(3), Some(2));
	}

	#[test]
	fn insert() {
		let mut range_set = RangeSet {
			ranges: vec![],
			total_size: 0,
		};
		range_set.insert(2);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![2..3],
				total_size: 1,
			}
		);
		range_set.insert(3);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![2..4],
				total_size: 2,
			}
		);
		range_set.insert(5);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![2..4, 5..6],
				total_size: 3,
			}
		);
		range_set.insert(4);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![2..6],
				total_size: 4,
			}
		);
		range_set.insert(8);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![2..6, 8..9],
				total_size: 5,
			}
		);
		range_set.insert(0);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![0..1, 2..6, 8..9],
				total_size: 6,
			}
		);
		range_set.insert(1);
		assert_eq!(
			range_set,
			RangeSet {
				ranges: vec![0..6, 8..9],
				total_size: 7,
			}
		);
	}
}
