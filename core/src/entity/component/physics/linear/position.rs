use crate::entity::component::{binary, debug, network, Component, Registration};
use engine::{
	math::nalgebra::{Point3, Vector3},
	utility::AnyError,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Position {
	chunk: Point3<i64>,
	offset: Point3<f32>,
	has_changed: bool,
}

impl Default for Position {
	fn default() -> Self {
		Self {
			chunk: Point3::new(0, 0, 0),
			offset: Point3::new(3.5, 0.0, 0.5),
			has_changed: false,
		}
	}
}

impl Component for Position {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::physics::linear::Position"
	}

	fn display_name() -> &'static str {
		"Position"
	}

	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		use binary::Registration as binary;
		use debug::Registration as debug;
		use network::Registration as network;
		Registration::<Self>::default()
			.with_ext(binary::from::<Self>())
			.with_ext(debug::from::<Self>())
			.with_ext(network::from::<Self>())
	}
}

impl std::fmt::Display for Position {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"Position(<{:04}`{:.2}, {:04}`{:.2}, {:04}`{:.2})",
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

impl std::ops::AddAssign<Vector3<f32>> for Position {
	fn add_assign(&mut self, rhs: Vector3<f32>) {
		use crate::common::world::chunk::SIZE;
		self.offset += rhs;
		let iter = self
			.offset
			.iter_mut()
			.zip(self.chunk.iter_mut())
			.zip(SIZE.iter());
		for ((offset, chunk), &size) in iter {
			let sign = if *offset < 0.0 {
				-1.0
			} else if *offset >= size {
				1.0
			} else {
				0.0
			};
			*offset -= sign * size;
			*chunk += sign as i64;
		}
		self.has_changed = true;
	}
}

impl network::Replicatable for Position {
	fn on_replication(&mut self, replicated: &Self, _is_locally_owned: bool) {
		*self = *replicated;
	}
}

impl binary::Serializable for Position {
	fn serialize(&self) -> Result<Vec<u8>, AnyError> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for Position {
	type Error = AnyError;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(binary::deserialize::<Self>(&bytes)?)
	}
}

impl debug::EguiInformation for Position {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!(
			"Chunk: <{:04}, {:04}, {:04}>",
			self.chunk[0], self.chunk[1], self.chunk[2]
		));
		ui.label(format!(
			"Offset: <{:.2}, {:.2}, {:.2}>",
			self.offset[0], self.offset[1], self.offset[2]
		));
	}
}
