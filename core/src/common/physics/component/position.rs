use crate::entity::component::{binary, debug, network, Component, Registration};
use anyhow::Result;
use engine::math::nalgebra::{Point3, Vector3};
use nalgebra::{Isometry3, Translation3, UnitQuaternion};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Position {
	prev_chunk: Option<Point3<i64>>,
	chunk: Point3<i64>,
	offset: Point3<f32>,
	has_changed: bool,
}

impl Default for Position {
	fn default() -> Self {
		Self {
			prev_chunk: None,
			chunk: Point3::new(0, 0, 0),
			offset: Point3::new(7.5, 8.0, 5.5),
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
			"Position(<{:04}`{:.2}, {:04}`{:.2}, {:04}`{:.2}>)",
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
	pub fn prev_chunk(&self) -> &Option<Point3<i64>> {
		&self.prev_chunk
	}

	pub fn acknowledge_chunk(&mut self) {
		self.prev_chunk = Some(self.chunk);
	}

	/// Returns the coordinate of the chunk the entity is in.
	pub fn chunk(&self) -> &Point3<i64> {
		&self.chunk
	}

	/// Returns the offset position the entity is at within their chunk.
	pub fn offset(&self) -> &Point3<f32> {
		&self.offset
	}

	/// Returns the physics translation required to move from origin to the current location.
	/// WARNING: This will result in a loss of precision at large values.
	pub fn translation(&self) -> Translation3<f32> {
		let chunk = self
			.chunk()
			.coords
			.cast::<f32>()
			.component_mul(&crate::common::world::chunk::SIZE);
		Translation3::from(self.offset() + chunk)
	}

	pub fn isometry(&self, orientation: Option<&super::Orientation>) -> Isometry3<f32> {
		Isometry3::from_parts(
			self.translation(),
			match orientation {
				Some(comp) => *comp.orientation(),
				None => UnitQuaternion::<f32>::identity(),
			},
		)
	}

	pub fn set_translation(&mut self, translation: Translation3<f32>) {
		let world = translation.vector;
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
		/*
		if is_locally_owned {
			let offset =
				(replicated.chunk - self.chunk).component_mul(&chunk::SIZE_I.cast::<i64>());
			let offset = (replicated.offset - self.offset) + offset.cast::<f32>();
			if offset.x < 0.25 && offset.y < 0.25 && offset.z < 0.25 {
				return;
			}
		}
		*/
		*self = *replicated;
	}
}

impl binary::Serializable for Position {
	fn serialize(&self) -> Result<Vec<u8>> {
		binary::serialize(&self)
	}
	fn deserialize(bytes: Vec<u8>) -> Result<Self> {
		binary::deserialize::<Self>(&bytes)
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
