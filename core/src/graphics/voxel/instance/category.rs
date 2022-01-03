use crate::block;

#[derive(Clone, Copy, Debug)]
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

	/// Adjusts the start index of the category by some amount.
	/// Does not change the expected size of the segment/index count.
	fn shift(&mut self, amount: i32) {
		let start = self.start as i32;
		assert!(start + amount >= 0);
		self.start = (start + amount) as usize;
	}

	/// Expands the amount of indices, adding some amount to the end bound.
	fn change_size(&mut self, amount: i32) {
		let count = self.count as i32;
		assert!(count + amount >= 0);
		self.count = (count + amount) as usize;
	}

	pub fn expand_right(&mut self) {
		self.change_size(1);
	}

	pub fn expand_left(&mut self) {
		self.change_size(1);
		self.shift(-1);
	}

	pub fn shrink_right(&mut self) {
		self.change_size(-1);
		self.shift(1);
	}

	pub fn shrink_left(&mut self) {
		self.change_size(-1);
	}
}
