use super::Registration;
use engine::{
	graphics::camera::{PerspectiveProjection, Projection},
	math::nalgebra::Vector3,
};

#[derive(Clone, Copy)]
pub struct Camera {
	view_offset: Vector3<f32>,
	format: Projection,
}

impl Default for Camera {
	fn default() -> Self {
		Self {
			view_offset: Vector3::new(0.0, 1.6, 0.0),
			format: Projection::Perspective(PerspectiveProjection {
				vertical_fov: 43.0,
				near_plane: 0.1,
				far_plane: 1000.0,
			}),
		}
	}
}

impl super::Component for Camera {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::Camera"
	}

	fn display_name() -> &'static str {
		"Camera"
	}

	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		Registration::<Self>::default().with_ext(super::debug::Registration::from::<Self>())
	}
}

impl std::fmt::Display for Camera {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"Camera(offset=<{}, {}, {}> format={})",
			self.view_offset[0], self.view_offset[1], self.view_offset[2], self.format
		)
	}
}

impl Camera {
	pub fn offset(&self) -> &Vector3<f32> {
		&self.view_offset
	}

	pub fn projection(&self) -> &Projection {
		&self.format
	}
}

impl super::debug::EguiInformation for Camera {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!(
			"Offset: <{:.2}, {:.2}, {:.2}>",
			self.view_offset[0], self.view_offset[1], self.view_offset[2]
		));
		match &self.format {
			Projection::Orthographic(ortho) => {
				ui.label("Projection: Orthographic");
				ui.label(format!("Left: {}", ortho.left()));
				ui.label(format!("Right: {}", ortho.right()));
				ui.label(format!("Top: {}", ortho.top()));
				ui.label(format!("Bottom: {}", ortho.bottom()));
				ui.label(format!("Z-Near: {}", ortho.z_near()));
				ui.label(format!("Z-Far: {}", ortho.z_far()));
			}
			Projection::Perspective(persp) => {
				ui.label("Projection: Perspective");
				ui.label(format!("Vertical FOV: {}", persp.vertical_fov));
				ui.label(format!("Z-Near: {}", persp.near_plane));
				ui.label(format!("Z-Far: {}", persp.far_plane));
			}
		}
	}
}
