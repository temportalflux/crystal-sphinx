use std::cmp::Ordering;
use std::ops::Range;

/// An ordered set of ranges.
/// Used to keep track of what indices have changed in a vec, without having a ginormous HashSet of usize indices.
#[derive(Default)]
pub struct RangeSet(Vec<Range<usize>>, usize);

impl RangeSet {
	#[profiling::function]
	pub fn insert(&mut self, idx: usize) {
		// Ok(index) means that a range was found which contains `idx`.
		// Err(index) means that no range currently contains `idx`,
		// but that the range at `index` could be expanded (start -= 1) if `idx` == `start - 1`
		// or range at `index - 1` could be expanded (end += 1) if `idx` == `end`.
		let result = self.0.binary_search_by(|range| -> Ordering {
			if range.end <= idx {
				return Ordering::Less;
			}
			if idx < range.start {
				return Ordering::Greater;
			}
			Ordering::Equal
		});
		let possible_range_idx = match result {
			// Some range contains the index already
			Ok(_range_idx) => return,
			Err(possible_range_idx) => possible_range_idx,
		};
		// The index is new, so the total count increments
		self.1 += 1;
		// Insert the new range, maintaining sort order
		self.0.insert(
			possible_range_idx,
			Range {
				start: idx,   // inclusive
				end: idx + 1, // exclusive
			},
		);
		self.merge_ranges_around(possible_range_idx);
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn take(&mut self) -> (Vec<Range<usize>>, usize) {
		let ranges = self.0.drain(..).collect();
		let total_count = self.1;
		self.1 = 0;
		(ranges, total_count)
	}

	/// Attempts to merge the range at `range_idx` with the one immediate preceeding and succeeding it.
	fn merge_ranges_around(&mut self, mut range_idx: usize) {
		// Try merge `range_idx - 1` into `range_idx`.
		if range_idx > 0 && self.can_merge(range_idx - 1, range_idx) {
			self.merge(range_idx - 1, range_idx);
			// If we merged, then the item at `range_idx` is now a part of `range_idx - 1`.
			range_idx = range_idx - 1;
		}

		// Try merge `range_idx` into `range_idx + 1`.
		if range_idx + 1 < self.0.len() && self.can_merge(range_idx, range_idx + 1) {
			self.merge(range_idx, range_idx + 1);
		}
	}

	fn can_merge(&self, r1_idx: usize, r2_idx: usize) -> bool {
		// Determine if both ranges exist (neither index is out of bounds)
		// and if the ranges are consecutive.
		match (self.0.get(r1_idx), self.0.get(r2_idx)) {
			(Some(r1), Some(r2)) => {
				// The ranges are consecutive if the end of the first (exclusive)
				// is equal to the start of the second (inclusive).
				r1.end == r2.start
			}
			_ => false,
		}
	}

	fn merge(&mut self, r1_idx: usize, r2_idx: usize) {
		let r2 = self.0.remove(r2_idx);
		let r1 = self.0.get_mut(r1_idx).unwrap();
		assert!(r1.start < r2.start);
		assert!(r1.end == r2.start);
		r1.end = r2.end;
	}
}
