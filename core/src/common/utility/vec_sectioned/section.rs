use super::{Operation, TargetPosition};
use std::ops::Range;

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Section {
	range: Range<usize>,
}

impl From<Range<usize>> for Section {
	fn from(range: Range<usize>) -> Self {
		Self { range }
	}
}

impl Section {
	pub fn new(start: usize, count: usize) -> Self {
		Self {
			range: Range {
				start,
				end: start + count,
			},
		}
	}

	pub fn start(&self) -> usize {
		self.range.start
	}

	pub fn end(&self) -> usize {
		self.range.end
	}

	pub fn count(&self) -> usize {
		assert!(self.range.end >= self.range.start);
		self.range.end - self.range.start
	}

	pub fn last(&self) -> usize {
		self.range.end - 1
	}

	pub fn is_empty(&self) -> bool {
		self.count() == 0
	}

	pub(super) fn inner(&self) -> &Range<usize> {
		&self.range
	}

	pub(super) fn apply(&mut self, operation: Operation) {
		match operation {
			Operation::Shift(size) => {
				let count = self.count();
				let start = self.range.start as i32;
				assert!(start + size.delta() >= 0);
				self.range.start = (start + size.delta()) as usize;
				self.range.end = self.range.start + count;
			}
			Operation::ChangeSize(size) => {
				self.range.end = ((self.range.end as i32) + size.delta()) as usize;
				assert!(self.range.end >= self.range.start);
			}
		}
	}

	pub(super) fn index_at_position(&self, pos: TargetPosition) -> usize {
		match pos {
			TargetPosition::First => self.start(),
			TargetPosition::Last => self.last(),
		}
	}
}

impl std::fmt::Debug for Section {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if self.count() > 0 {
			write!(
				f,
				"Section([{},{})#{})",
				self.start(),
				self.range.end,
				self.count()
			)
		} else {
			write!(f, "Section({}..Ø[0])", self.start())
		}
	}
}

#[cfg(test)]
mod section {
	use super::*;

	#[test]
	fn default() {
		assert_eq!(Section::default(), Section { range: 0..0 });
	}

	#[test]
	fn apply_shift_positive() {
		let mut section = Section { range: 3..5 };
		section.apply(Operation::Shift(OperationSize::Increment));
		assert_eq!(section, Section { range: 4..6 });
	}

	#[test]
	fn apply_shift_negative() {
		let mut section = Section { range: 3..5 };
		section.apply(Operation::Shift(OperationSize::Decrement));
		assert_eq!(section, Section { range: 2..4 });
	}

	#[test]
	fn apply_resize_increase() {
		let mut section = Section { range: 3..5 };
		section.apply(Operation::ChangeSize(OperationSize::Increment));
		assert_eq!(section, Section { range: 3..6 });
	}

	#[test]
	fn apply_resize_decrease() {
		let mut section = Section { range: 3..5 };
		section.apply(Operation::ChangeSize(OperationSize::Decrement));
		assert_eq!(section, Section { range: 3..4 });
	}
}
