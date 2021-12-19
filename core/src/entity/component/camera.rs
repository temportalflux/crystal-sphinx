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
			view_offset: Vector3::new(0.0, 0.0, 0.0),
			format: Projection::Perspective(PerspectiveProjection {
				vertical_fov: 45.0,
				near_plane: 0.1,
				far_plane: 1000.0,
			}),
		}
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
