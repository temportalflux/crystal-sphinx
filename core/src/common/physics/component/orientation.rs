use crate::entity::component::{binary, debug, network, Component, Registration};
use anyhow::Result;
use engine::{
	math::nalgebra::{Unit, UnitQuaternion, Vector3},
	world,
};
use nalgebra::{Isometry3, Translation3};
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

impl Component for Orientation {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::Orientation"
	}

	fn display_name() -> &'static str {
		"Orientation"
	}

	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		Registration::<Self>::default()
			.with_ext(binary::Registration::from::<Self>())
			.with_ext(debug::Registration::from::<Self>())
			.with_ext(network::Registration::from::<Self>())
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

	pub fn isometry(&self) -> Isometry3<f32> {
		Isometry3::from_parts(Translation3::default(), *self.orientation())
	}

	pub fn set_rotation(&mut self, rotation: UnitQuaternion<f32>) {
		self.0 = rotation;
	}

	pub fn forward(&self) -> Unit<Vector3<f32>> {
		self.orientation() * world::global_forward()
	}
}

impl network::Replicatable for Orientation {
	fn on_replication(&mut self, replicated: &Self, is_locally_owned: bool) {
		if !is_locally_owned {
			*self = *replicated;
		}
	}
}

impl binary::Serializable for Orientation {
	fn serialize(&self) -> Result<Vec<u8>> {
		binary::serialize(&self)
	}
	fn deserialize(bytes: Vec<u8>) -> Result<Self> {
		binary::deserialize::<Self>(&bytes)
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

impl debug::EguiInformation for Orientation {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(match self.0.axis() {
			Some(axis) => {
				format!("Axis: <{:.2}, {:.2}, {:.2}>", axis[0], axis[1], axis[2])
			}
			None => "None".to_owned(),
		});
		ui.label(format!("Angle: {}°", self.0.angle().to_degrees()));
	}
}
