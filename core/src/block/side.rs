use crate::graphics::voxel::Face;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, Serialize, Deserialize)]
pub enum Side {
	Top,
	Bottom,
	Front,
	Back,
	Left,
	Right,

	Side,
}
impl Side {
	pub fn all_real() -> Vec<Self> {
		vec![
			Self::Top,
			Self::Bottom,
			Self::Front,
			Self::Back,
			Self::Left,
			Self::Right,
		]
	}

	pub fn all() -> Vec<Self> {
		vec![
			Self::Top,
			Self::Bottom,
			Self::Front,
			Self::Back,
			Self::Left,
			Self::Right,
			Self::Side,
		]
	}

	pub fn as_side_list(&self) -> Vec<Self> {
		match self {
			Self::Side => vec![Self::Front, Self::Back, Self::Left, Self::Right],
			_ => vec![*self],
		}
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Top => "Top",
			Self::Bottom => "Bottom",
			Self::Front => "Front",
			Self::Back => "Back",
			Self::Left => "Left",
			Self::Right => "Right",
			Self::Side => "Side",
		}
	}
}

impl std::convert::TryFrom<&str> for Side {
	type Error = ();
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value {
			"Top" => Ok(Self::Top),
			"Bottom" => Ok(Self::Bottom),
			"Front" => Ok(Self::Front),
			"Back" => Ok(Self::Back),
			"Left" => Ok(Self::Left),
			"Right" => Ok(Self::Right),
			"Side" => Ok(Self::Side),
			_ => Err(()),
		}
	}
}

impl Into<Face> for Side {
	fn into(self) -> Face {
		match self {
			Self::Top => Face::Up,
			Self::Bottom => Face::Down,
			Self::Front => Face::Front,
			Self::Back => Face::Back,
			Self::Left => Face::Left,
			Self::Right => Face::Right,
			Self::Side => unimplemented!(),
		}
	}
}
