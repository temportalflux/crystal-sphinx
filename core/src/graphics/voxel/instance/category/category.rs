use super::{Operation, TargetPosition};
use crate::block;

#[derive(Clone, Copy)]
pub struct Category {
	pub(crate) id: Option<block::LookupId>,
	start: usize,
	count: usize,
}

impl Category {
	pub fn new(id: Option<block::LookupId>, count: usize) -> Self {
		Self {
			id,
			start: 0,
			count,
		}
	}

	pub fn start(&self) -> usize {
		self.start
	}

	pub fn count(&self) -> usize {
		self.count
	}

	pub fn last(&self) -> usize {
		self.start + self.count - 1
	}

	pub fn apply(&mut self, operation: Operation) {
		match operation {
			Operation::Shift(delta) => {
				let start = self.start as i32;
				assert!(start + delta >= 0);
				self.start = (start + delta) as usize;
			}
			Operation::ChangeSize(delta) => {
				let count = self.count as i32;
				assert!(count + delta >= 0);
				self.count = (count + delta) as usize;
			}
		}
	}

	pub fn index_at_position(&self, pos: TargetPosition) -> usize {
		match pos {
			TargetPosition::First => self.start(),
			TargetPosition::Last => self.last(),
		}
	}
}

impl std::fmt::Debug for Category {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if self.count() > 0 {
			write!(
				f,
				"Category({:?} : {}..{} ({}))",
				self.id,
				self.start(),
				self.start + self.count,
				self.count()
			)
		} else {
			write!(f, "Category({:?} : {}..Ø (0))", self.id, self.start())
		}
	}
}
