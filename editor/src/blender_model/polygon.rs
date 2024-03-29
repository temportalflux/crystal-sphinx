use engine::math::nalgebra::{Vector2, Vector3};
use tokio::{io::AsyncReadExt, process::ChildStdout};

#[derive(Debug)]
pub struct Polygon {
	pub normal: Vector3<f32>,
	pub vertices: Vec<(usize, Vector2<f32>)>,
}

impl Polygon {
	pub async fn read(stream: &mut ChildStdout) -> anyhow::Result<Self> {
		let normal = {
			let x = stream.read_f32().await?;
			let y = stream.read_f32().await?;
			let z = stream.read_f32().await?;
			Vector3::new(y, z, -x)
		};

		let index_count = stream.read_u32().await? as usize;
		let mut vertices = Vec::with_capacity(index_count);
		for _ in 0..index_count {
			let vertex_index = stream.read_u32().await? as usize;

			let uv = {
				let x = stream.read_f32().await?;
				let y = stream.read_f32().await?;
				// Blender uv origin is bottom-left, not top-left.
				// We need to flip the y coordinate b/c engine uses top-left.
				Vector2::new(x, 1.0 - y)
			};

			vertices.push((vertex_index, uv));
		}

		Ok(Self { normal, vertices })
	}
}
