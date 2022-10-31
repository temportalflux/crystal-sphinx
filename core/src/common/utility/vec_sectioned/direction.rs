use super::{Operation, OperationSize, TargetPosition};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
	/// The destination category is to the left in the buffer memory of the start.
	Left,
	/// The destination category is to the right in the buffer memory of the start.
	Right,
}

impl Direction {
	pub fn from<K>(start: &K, destination: &K) -> Self
	where
		K: Ord,
	{
		use std::cmp::Ordering;
		// The idx offset from the start of the target category that the instance should be swapped into.
		match destination.cmp(&start) {
			Ordering::Less => Self::Left,
			Ordering::Greater => Self::Right,
			_ => unimplemented!(),
		}
	}

	pub(super) fn operations<K>(&self, prev: K, next: K) -> (Vec<(K, Operation)>, TargetPosition)
	where
		K: Copy,
	{
		match self {
			Self::Left => {
				(
					vec![
						// Shrink to the right
						(prev, Operation::ChangeSize(OperationSize::Decrement)),
						(prev, Operation::Shift(OperationSize::Increment)),
						// Expand to the right
						(next, Operation::ChangeSize(OperationSize::Increment)),
					],
					// The last item in `next` will be in the `next` section once operations are complete.
					TargetPosition::Last,
				)
			}
			Self::Right => {
				(
					vec![
						// Shrink to the left
						(prev, Operation::ChangeSize(OperationSize::Decrement)),
						// Expand to the left
						(next, Operation::ChangeSize(OperationSize::Increment)),
						(next, Operation::Shift(OperationSize::Decrement)),
					],
					// The first item in `next` will be in the `next` section once operations are complete.
					TargetPosition::First,
				)
			}
		}
	}

	pub(super) fn target_position(&self) -> TargetPosition {
		match self {
			Self::Left => TargetPosition::First,
			Self::Right => TargetPosition::Last,
		}
	}
}

impl std::ops::Add<Direction> for usize {
	type Output = usize;

	fn add(self, rhs: Direction) -> Self::Output {
		match rhs {
			Direction::Left => self - 1,
			Direction::Right => self + 1,
		}
	}
}

impl std::ops::AddAssign<Direction> for usize {
	fn add_assign(&mut self, rhs: Direction) {
		*self = *self + rhs;
	}
}
