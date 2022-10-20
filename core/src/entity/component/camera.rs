use super::Registration;
use engine::{
	graphics::camera::{PerspectiveProjection, Projection},
	math::{
		self,
		nalgebra::{Isometry3, UnitQuaternion, Vector3},
	},
	world,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct Camera {
	view: CameraView,
	format: Projection,
}

impl Default for Camera {
	fn default() -> Self {
		Self {
			view: CameraView::ThirdPersonBack,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Perspective {
	FirstPerson,
	ThirdPerson,
}

impl CameraView {
	/// Return the camera perspective's translation and rotation for a given player orientation.
	pub fn get_isometry(&self, orientation: &UnitQuaternion<f32>) -> Isometry3<f32> {
		let eye_offset = Vector3::<f32>::new(0.0, 1.6, 0.0);
		let third_person_offset = 5.0;
		match self {
			Self::FirstPerson => Isometry3::from_parts(eye_offset.into(), *orientation),
			Self::ThirdPersonBack => {
				let player_backward: Vector3<f32> = orientation * -*world::global_forward();
				Isometry3::from_parts(
					(eye_offset + (player_backward * third_person_offset)).into(),
					*orientation,
				)
			}
			Self::ThirdPersonFront => {
				let player_forward: Vector3<f32> = orientation * *world::global_forward();
				let rotation = math::face_towards_rh(&-player_forward, &*world::global_up());
				Isometry3::from_parts(
					(eye_offset + (player_forward * third_person_offset)).into(),
					rotation,
				)
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

	pub fn perspective(&self) -> Perspective {
		match self {
			Self::FirstPerson => Perspective::FirstPerson,
			Self::ThirdPersonBack | Self::ThirdPersonFront => Perspective::ThirdPerson,
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
