
mod category;
pub use category::*;
mod direction;
pub use direction::*;
mod key;
pub use key::*;

pub enum Operation
{
	/// Adjusts the start index of the category by some amount.
	/// Does not change the expected size of the segment/index count.
	Shift(i32),
	/// Expands the amount of indices, adding some amount to the end bound.
	ChangeSize(i32),
}

pub enum TargetPosition {
	First,
	Last,
}
