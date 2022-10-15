pub use camera::{OrthographicBounds, PerspectiveProjection, Projection};
use engine::{
	graphics::camera,
	math::nalgebra::{
		self, point, Isometry3, Matrix4, Point3, Translation3, UnitQuaternion, Vector2,
	},
};
use std::sync::{Arc, RwLock};

pub type ArcLockCamera = Arc<RwLock<Camera>>;
#[derive(Clone)]
pub struct Camera {
	pub chunk_coordinate: Point3<f32>,
	pub position: Point3<f32>,
	pub orientation: UnitQuaternion<f32>,
	pub projection: camera::Projection,
}

impl Default for Camera {
	fn default() -> Self {
		Self {
			chunk_coordinate: Point3::<f32>::new(0.0, 0.0, 0.0),
			position: Point3::<f32>::new(0.0, 0.0, 0.0),
			orientation: UnitQuaternion::identity(),
			projection: camera::Projection::Perspective(camera::PerspectiveProjection {
				vertical_fov: 43.0,
				near_plane: 0.1,
				far_plane: 1000.0,
			}),
		}
	}
}

impl camera::Camera for Camera {
	fn position(&self) -> &Point3<f32> {
		&self.position
	}
	fn orientation(&self) -> &UnitQuaternion<f32> {
		&self.orientation
	}
	fn projection(&self) -> &camera::Projection {
		&self.projection
	}
}

impl Camera {
	pub fn as_uniform_data(&self, resolution: &Vector2<f32>) -> UniformData {
		use camera::Camera;
		let inv_rotation = {
			let no_offset = Translation3::new(0.0, 0.0, 0.0);
			let rot_camera_to_world = self.orientation.inverse();
			let iso = Isometry3::from_parts(no_offset, rot_camera_to_world);
			iso.to_homogeneous()
		};
		UniformData {
			view: self.view_matrix(),
			projection: self.projection_matrix(resolution),
			chunk_coordinate: self.chunk_coordinate,
			inv_rotation,
		}
	}
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct UniformData {
	view: Matrix4<f32>,
	projection: Matrix4<f32>,
	inv_rotation: Matrix4<f32>,
	chunk_coordinate: Point3<f32>,
}

impl Default for UniformData {
	fn default() -> Self {
		Self {
			view: Matrix4::identity(),
			projection: Matrix4::identity(),
			chunk_coordinate: point![0.0, 0.0, 0.0],
			inv_rotation: Matrix4::identity(),
		}
	}
}
