use engine::{math::nalgebra::UnitQuaternion, utility::Result};
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

	fn registration() -> super::Registration<Self>
	where
		Self: Sized,
	{
		use super::binary::Registration as binary;
		use super::debug::Registration as debug;
		use super::network::Registration as network;
		super::Registration::<Self>::default()
			.with_ext(binary::from::<Self>())
			.with_ext(debug::from::<Self>())
			.with_ext(network::from::<Self>())
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

impl super::network::Replicatable for Orientation {
	fn on_replication(&mut self, replicated: &Self, is_locally_owned: bool) {
		if !is_locally_owned {
			*self = *replicated;
		}
	}
}

impl super::binary::Serializable for Orientation {
	fn serialize(&self) -> Result<Vec<u8>> {
		super::binary::serialize(&self)
	}
	fn deserialize(bytes: Vec<u8>) -> Result<Self> {
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

impl super::debug::EguiInformation for Orientation {
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
