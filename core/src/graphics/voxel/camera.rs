use engine::{
	graphics::camera,
	math::nalgebra::{self, point, vector, Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
};
use std::sync::{Arc, RwLock};

pub use camera::{OrthographicBounds, PerspectiveProjection, Projection};

pub type ArcLockCamera = Arc<RwLock<Camera>>;
#[derive(Clone)]
pub struct Camera {
	pub chunk_size: Vector3<f32>,
	pub chunk_coordinate: Point3<f32>,
	pub position: Point3<f32>,
	pub orientation: UnitQuaternion<f32>,
	pub projection: camera::Projection,
}

impl Default for Camera {
	fn default() -> Self {
		Self {
			chunk_size: Vector3::<f32>::new(16.0, 16.0, 16.0),
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
		UniformData {
			view: self.view_matrix(),
			projection: self.projection_matrix(resolution),
			chunk_coordinate: self.chunk_coordinate,
			chunk_size: self.chunk_size,
		}
	}
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct UniformData {
	view: Matrix4<f32>,
	projection: Matrix4<f32>,
	chunk_coordinate: Point3<f32>,
	chunk_size: Vector3<f32>,
}

impl Default for UniformData {
	fn default() -> Self {
		Self {
			view: Matrix4::identity(),
			projection: Matrix4::identity(),
			chunk_coordinate: point![0.0, 0.0, 0.0],
			chunk_size: vector![16.0, 16.0, 16.0],
		}
	}
}
