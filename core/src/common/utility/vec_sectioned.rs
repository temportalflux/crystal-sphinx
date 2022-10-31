mod direction;
use direction::*;
mod range_set;
pub use range_set::*;
mod section;
use section::*;
mod vec;
pub use vec::*;

enum Operation {
	/// Adjusts the start index of the category by some amount.
	/// Does not change the expected size of the segment/index count.
	Shift(OperationSize),
	/// Expands the amount of indices, adding some amount to the end bound.
	ChangeSize(OperationSize),
}

enum OperationSize {
	Increment,
	Decrement,
}
impl OperationSize {
	fn delta(&self) -> i32 {
		match self {
			Self::Increment => 1,
			Self::Decrement => -1,
		}
	}
}

enum TargetPosition {
	First,
	Last,
}
