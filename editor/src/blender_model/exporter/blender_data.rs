use engine::task::JoinHandle;
use futures_util::future::Future;
use std::path::PathBuf;

use crate::exporter::Model;
pub struct BlenderData;

impl BlenderData {	
	pub async fn from_stream(mut stream: tokio::process::ChildStdout) -> anyhow::Result<Self> {
		use tokio::io::*;

		while let Ok(byte) = stream.read_u8().await {
			if byte == 0b00 {
				// Found start of data stream
				break;
			}
		}

		let vertex_count = stream.read_u32().await? as usize;
		let mut vertices = Vec::with_capacity(vertex_count);
		for _ in 0..vertex_count {
			let pos_x = stream.read_f32().await?;
			let pos_y = stream.read_f32().await?;
			let pos_z = stream.read_f32().await?;

			let group_count = stream.read_u32().await? as usize;
			let mut groups = Vec::with_capacity(group_count);
			for _ in 0..group_count {
				let group_id = stream.read_u32().await? as usize;
				let weight = stream.read_f32().await?;
				groups.push((group_id, weight));
			}

			vertices.push(((pos_x, pos_y, pos_z), groups));
		}

		let polygon_count = stream.read_u32().await? as usize;
		for _ in 0..polygon_count {
			let normal_x = stream.read_f32().await?;
			let normal_y = stream.read_f32().await?;
			let normal_z = stream.read_f32().await?;

			let index_count = stream.read_u32().await? as usize;
			for _ in 0..index_count {
				let vertex_index = stream.read_u32().await? as usize;
			}

			let loop_idx_start = stream.read_u32().await? as usize;
			let loop_idx_end = stream.read_u32().await? as usize;
			let loop_range = loop_idx_start..loop_idx_end;
		}

		let loop_count = stream.read_u32().await? as usize;
		for idx in 0..loop_count {
			let vertex_index = stream.read_u32().await? as usize;
			let uv_x = stream.read_f32().await?;
			let uv_y = stream.read_f32().await?;
		}

		let end_byte = stream.read_u8().await;
		assert_eq!(end_byte.ok(), Some(0b00));

		Ok(Self)
	}
	
	pub fn into_model(self) -> anyhow::Result<Model> {
		Ok(Model)
	}
}
