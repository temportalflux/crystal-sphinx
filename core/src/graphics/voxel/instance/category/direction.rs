use super::{Key, Operation, TargetPosition};

#[derive(Clone, Copy, Debug)]
pub enum Direction {
	/// The destination category is to the left in the buffer memory of the start.
	Left,
	/// The destination category is to the right in the buffer memory of the start.
	Right,
}

impl Direction {
	pub fn from(start: &Key, destination: &Key) -> Self {
		use std::cmp::Ordering;
		// The idx offset from the start of the target category that the instance should be swapped into.
		match destination.cmp(&start) {
			Ordering::Less => Self::Left,
			Ordering::Greater => Self::Right,
			_ => unimplemented!(),
		}
	}

	pub fn operations(&self, prev: Key, next: Key) -> Vec<(Key, Operation)> {
		match self {
			Self::Left => {
				vec![
					// Shrink to the right
					(prev, Operation::ChangeSize(-1)),
					(prev, Operation::Shift(1)),
					// Expand to the right
					(next, Operation::ChangeSize(1)),
				]
			}
			Self::Right => {
				vec![
					// Shrink to the left
					(prev, Operation::ChangeSize(-1)),
					// Expand to the left
					(next, Operation::ChangeSize(1)),
					(next, Operation::Shift(-1)),
				]
			}
		}
	}

	pub fn target_position(&self) -> TargetPosition {
		match self {
			Self::Left => TargetPosition::First,
			Self::Right => TargetPosition::Last,
		}
	}
}
