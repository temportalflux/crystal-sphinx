use engine::{math::nalgebra::UnitQuaternion, utility::AnyError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Orientation(UnitQuaternion<f32>);

impl Default for Orientation {
	fn default() -> Self {
		//Self(UnitQuaternion::identity())
		Self(UnitQuaternion::from_axis_angle(
			&-engine::world::global_up(),
			-90.0f32.to_radians(),
		))
	}
}

impl super::Component for Orientation {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::Orientation"
	}

	fn display_name() -> &'static str {
		"Orientation"
	}
}

impl std::fmt::Display for Orientation {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self.0.axis() {
			Some(axis) => write!(
				f,
				"Orientation(<{}, {}, {}> @ {})",
				axis[0],
				axis[1],
				axis[2],
				self.0.angle().to_degrees()
			),
			None => write!(f, "Orientation(None)"),
		}
	}
}

impl Orientation {
	pub fn orientation(&self) -> &UnitQuaternion<f32> {
		&self.0
	}
}

impl super::binary::Serializable for Orientation {
	fn serialize(&self) -> Result<Vec<u8>, AnyError> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for Orientation {
	type Error = rmp_serde::decode::Error;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		super::binary::deserialize::<Self>(&bytes)
	}
}

impl std::ops::Deref for Orientation {
	type Target = UnitQuaternion<f32>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for Orientation {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
