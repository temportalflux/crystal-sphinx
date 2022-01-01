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

	pub fn as_face_set(&self) -> enumset::EnumSet<Face> {
		match self {
			Self::Side => Face::Front | Face::Back | Face::Left | Face::Right,
			_ => Face::from(*self).into(),
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
