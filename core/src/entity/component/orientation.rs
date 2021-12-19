use engine::math::nalgebra::UnitQuaternion;

#[derive(Clone, Copy)]
pub struct Orientation(UnitQuaternion<f32>);

impl Default for Orientation {
	fn default() -> Self {
		Self(UnitQuaternion::identity())
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
