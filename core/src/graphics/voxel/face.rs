use engine::{
	math::nalgebra::{Matrix3x4, Vector3},
	world::{global_forward, global_right, global_up},
};
use enumset::{EnumSet, EnumSetType};

#[derive(Debug, Hash, EnumSetType)]
pub enum Face {
	Right,
	Left,
	Up,
	Down,
	Front,
	Back,
}

impl std::fmt::Display for Face {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Left => "Left",
				Self::Right => "Right",
				Self::Down => "Down",
				Self::Up => "Up",
				Self::Front => "Front",
				Self::Back => "Back",
			}
		)
	}
}

impl Face {
	#[rustfmt::skip]
	pub fn model_bit(&self) -> u32 {
		match self {
			Self::Left =>  0b000001,
			Self::Right => 0b000010,
			Self::Down =>  0b000100,
			Self::Up =>    0b001000,
			Self::Front => 0b010000,
			Self::Back =>  0b100000,
		}
	}

	pub fn parse_model_bit(faces_enabled_bitfield: u32) -> EnumSet<Face> {
		let mut faces = EnumSet::empty();
		for face in EnumSet::<Face>::all().iter() {
			let bit_mask = face.model_bit();
			if faces_enabled_bitfield & bit_mask == bit_mask {
				faces |= face;
			}
		}
		faces
	}

	/// Returns a vector representing what is considered the "up" direction for determining the face's vertex positions.
	fn up(&self) -> Vector3<f32> {
		match self {
			Self::Right | Self::Left => *global_up(),
			Self::Front | Self::Back => *global_up(),
			// Reads -Z to +Z (front to back)
			Self::Up | Self::Down => Default::default(), // zero
		}
	}

	/// Returns a vector representing what is considered the "down" direction for determining the face's vertex positions.
	fn down(&self) -> Vector3<f32> {
		match self {
			Self::Right | Self::Left => Default::default(), // zero
			Self::Front | Self::Back => Default::default(), // zero
			// Reads -Z to +Z (front to back)
			Self::Up | Self::Down => -*global_forward(),
		}
	}

	/// Returns a vector representing what is considered the "left" direction for determining the face's vertex positions.
	fn left(&self) -> Vector3<f32> {
		match self {
			// Reads -Z to +Z (front to back)
			Self::Right => Default::default(), // zero
			// Reads +Z to -Z (back to front)
			Self::Left => -*global_forward(),
			// Reads -X to +X (left to right)
			Self::Front => Default::default(), // zero
			// Reads +X to -X (right to left)
			Self::Back => *global_right(),
			Self::Down => Default::default(), // zero
			Self::Up => *global_right(),
		}
	}

	/// Returns a vector representing what is considered the "right" direction for determining the face's vertex positions.
	fn right(&self) -> Vector3<f32> {
		match self {
			// Reads -Z to Z (front to back)
			Self::Right => -*global_forward(),
			// Reads +Z to -Z (back to front)
			Self::Left => Default::default(), // zero
			// Reads -X to +X (left to right)
			Self::Front => *global_right(),
			// Reads +X to -X (right to left)
			Self::Back => Default::default(), // zero
			Self::Down => *global_right(),
			Self::Up => Default::default(), // zero
		}
	}

	/// Returns the Top/Bottom Left/Right locations of the face.
	/// This determines the tangent of the face for the rendering of a uniform voxel.
	pub fn model_offset_matrix(&self) -> Matrix3x4<f32> {
		Matrix3x4::from_columns(&[self.left(), self.right(), self.up(), self.down()])
	}

	pub fn model_axis(&self) -> Vector3<f32> {
		match self {
			Self::Left => Vector3::default(),
			Self::Right => *global_right(),
			Self::Down => Vector3::default(),
			Self::Up => *global_up(),
			Self::Front => Vector3::default(),
			Self::Back => *-global_forward(),
		}
	}

	pub fn direction(&self) -> Vector3<i8> {
		match self {
			Self::Left => Vector3::new(-1, 0, 0),
			Self::Right => Vector3::new(1, 0, 0),
			Self::Down => Vector3::new(0, -1, 0),
			Self::Up => Vector3::new(0, 1, 0),
			Self::Front => Vector3::new(0, 0, -1),
			Self::Back => Vector3::new(0, 0, 1),
		}
	}

	pub fn inverse(&self) -> Self {
		match self {
			Self::Left => Self::Right,
			Self::Right => Self::Left,
			Self::Down => Self::Up,
			Self::Up => Self::Down,
			Self::Front => Self::Back,
			Self::Back => Self::Front,
		}
	}
}

impl From<crate::block::Side> for Face {
	fn from(side: crate::block::Side) -> Self {
		use crate::block::Side;
		match side {
			Side::Top => Self::Up,
			Side::Bottom => Self::Down,
			Side::Front => Self::Front,
			Side::Back => Self::Back,
			Side::Left => Self::Left,
			Side::Right => Self::Right,
			Side::Side => unimplemented!(),
		}
	}
}
