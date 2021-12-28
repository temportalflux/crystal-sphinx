use engine::{
	math::nalgebra::{Matrix3x4, Vector3},
	world::{global_forward, global_right, global_up},
};

#[derive(Debug, Hash, enumset::EnumSetType)]
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
	pub fn model_bit(&self) -> u32 {
		match self {
			Self::Left => 0b000001,
			Self::Right => 0b000010,
			Self::Down => 0b000100,
			Self::Up => 0b001000,
			Self::Front => 0b010000,
			Self::Back => 0b100000,
		}
	}

	#[rustfmt::skip]
	pub fn model_offset_matrix(&self) -> Matrix3x4<f32> {
		let zero = Vector3::<f32>::default();
		match self {
			Self::Left => Matrix3x4::from_columns(&[*-global_forward(), zero, *global_up(), zero]),
			Self::Right => Matrix3x4::from_columns(&[zero, *-global_forward(), *global_up(), zero]),
			Self::Down => Matrix3x4::from_columns(&[zero, *global_right(), zero, *-global_forward()]),
			Self::Up => Matrix3x4::from_columns(&[zero, *global_right(), *-global_forward(), zero]),
			Self::Front => Matrix3x4::from_columns(&[zero, *global_right(), *global_up(), zero]),
			Self::Back => Matrix3x4::from_columns(&[*global_right(), zero, *global_up(), zero]),
		}
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
