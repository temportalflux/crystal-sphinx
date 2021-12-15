use engine::math::nalgebra::{self, point, vector, Matrix4, Point3, Vector3};

#[derive(Debug, Clone, Copy)]
pub struct ViewProjection {
	pub view: Matrix4<f32>,
	pub projection: Matrix4<f32>,
	pub chunk_coordinate: Point3<f32>,
	pub chunk_size: Vector3<f32>,
}

impl Default for ViewProjection {
	fn default() -> Self {
		Self {
			view: Matrix4::identity(),
			projection: Matrix4::identity(),
			chunk_coordinate: point![0.0, 0.0, 0.0],
			chunk_size: vector![16.0, 16.0, 16.0],
		}
	}
}
