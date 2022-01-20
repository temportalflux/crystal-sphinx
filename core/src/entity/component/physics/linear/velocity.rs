use crate::entity::component::{binary, debug, network, Component, Registration};
use engine::{math::nalgebra::Vector3, utility::{Result, Error}};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Velocity(Vector3<f32>);

impl Default for Velocity {
	fn default() -> Self {
		Self(Vector3::new(0.0, 0.0, 0.0))
	}
}

impl Component for Velocity {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::physics::linear::Velocity"
	}

	fn display_name() -> &'static str {
		"Velocity"
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

impl std::fmt::Display for Velocity {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"Velocity(<{:.2}, {:.2}, {:.2}>)",
			self.0[0], self.0[1], self.0[2]
		)
	}
}

impl std::ops::Deref for Velocity {
	type Target = Vector3<f32>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for Velocity {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl network::Replicatable for Velocity {
	fn on_replication(&mut self, replicated: &Self, _is_locally_owned: bool) {
		*self = *replicated;
	}
}

impl binary::Serializable for Velocity {
	fn serialize(&self) -> Result<Vec<u8>> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for Velocity {
	type Error = Error;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(binary::deserialize::<Self>(&bytes)?)
	}
}

impl debug::EguiInformation for Velocity {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!(
			"<{:.2}, {:.2}, {:.2}>",
			self.0[0], self.0[1], self.0[2]
		));

		let direction = self.0.normalize();
		let speed = self.0.magnitude();
		ui.label(format!(
			"Direction: <{:.2}, {:.2}, {:.2}>",
			direction[0], direction[1], direction[2]
		));
		ui.label(format!("Speed: {:.4}", speed));
	}
}
