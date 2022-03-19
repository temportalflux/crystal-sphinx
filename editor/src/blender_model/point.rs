use engine::math::nalgebra::Vector3;
use tokio::{io::AsyncReadExt, process::ChildStdout};

pub struct Point {
	pub position: Vector3<f32>,
	pub groups: Vec<WeightedGroup>,
}

pub struct WeightedGroup {
	pub group_index: usize,
	pub weight: f32,
}

impl Point {
	pub async fn read(stream: &mut ChildStdout) -> anyhow::Result<Self> {
		let position = {
			let x = stream.read_f32().await?;
			let y = stream.read_f32().await?;
			let z = stream.read_f32().await?;
			Vector3::new(x, y, z)
		};

		let group_count = stream.read_u32().await? as usize;
		let mut groups = Vec::with_capacity(group_count);
		for _ in 0..group_count {
			let group_index = stream.read_u32().await? as usize;
			let weight = stream.read_f32().await?;
			groups.push(WeightedGroup {
				group_index,
				weight,
			});
		}

		Ok(Self { position, groups })
	}
}
