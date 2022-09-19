use super::Registration;
use engine::{
	graphics::camera::{PerspectiveProjection, Projection},
	math::nalgebra::{UnitQuaternion, Vector3},
	world,
};

#[derive(Clone, Copy)]
pub struct Camera {
	view: CameraView,
	format: Projection,
}

impl Default for Camera {
	fn default() -> Self {
		Self {
			view: CameraView::FirstPerson,
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
		write!(f, "Camera(view={:?} format={})", self.view, self.format)
	}
}

impl Camera {
	pub fn view(&self) -> &CameraView {
		&self.view
	}

	pub fn set_view(&mut self, view: CameraView) {
		self.view = view;
	}

	pub fn projection(&self) -> &Projection {
		&self.format
	}
}

#[derive(Clone, Copy, Debug)]
pub enum CameraView {
	FirstPerson,
	ThirdPersonBack,
	ThirdPersonFront,
}

impl CameraView {
	pub fn offset(&self) -> Vector3<f32> {
		match self {
			Self::FirstPerson => Vector3::new(0.0, 1.6, 0.0),
			Self::ThirdPersonBack => Vector3::new(0.0, 1.6, 5.0),
			Self::ThirdPersonFront => Vector3::new(0.0, 1.6, -5.0),
		}
	}

	pub fn orientation(&self) -> UnitQuaternion<f32> {
		match self {
			Self::FirstPerson => UnitQuaternion::identity(),
			Self::ThirdPersonBack => UnitQuaternion::identity(),
			Self::ThirdPersonFront => {
				UnitQuaternion::from_axis_angle(&world::global_up(), std::f32::consts::PI)
			}
		}
	}

	pub fn next(&self) -> Self {
		match self {
			Self::FirstPerson => Self::ThirdPersonBack,
			Self::ThirdPersonBack => Self::ThirdPersonFront,
			Self::ThirdPersonFront => Self::FirstPerson,
		}
	}
}

impl super::debug::EguiInformation for Camera {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!("View: {:?}", self.view));
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
