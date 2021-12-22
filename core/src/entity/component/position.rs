use super::net;
use engine::{math::nalgebra::Point3, utility::AnyError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Position {
	chunk: Point3<i64>,
	pub(crate) offset: Point3<f32>,
}

impl Default for Position {
	fn default() -> Self {
		Self {
			chunk: Point3::new(0, 0, 0),
			offset: Point3::new(0.0, 0.0, 0.0),
		}
	}
}

impl std::fmt::Display for Position {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"Position(<{}`{}, {}`{}, {}`{}>)",
			self.chunk[0],
			self.offset[0],
			self.chunk[1],
			self.offset[1],
			self.chunk[2],
			self.offset[2]
		)
	}
}

impl Position {
	/// Returns the coordinate of the chunk the entity is in.
	pub fn chunk(&self) -> &Point3<i64> {
		&self.chunk
	}

	/// Returns the offset position the entity is at within their chunk.
	pub fn offset(&self) -> &Point3<f32> {
		&self.offset
	}
}

impl net::Replicated for Position {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::Position"
	}

	fn serialize(&self) -> Result<Vec<u8>, AnyError> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for Position {
	type Error = AnyError;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(net::deserialize::<Self>(&bytes)?)
	}
}
