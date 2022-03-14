use crate::exporter::{ExportError, Model};
use tokio::{io::AsyncReadExt, process::ChildStdout};

pub struct BlenderData {
	stream: ChildStdout,
}

impl BlenderData {
	pub fn new(stream: ChildStdout) -> Self {
		Self { stream }
	}

	pub async fn process(mut self) -> anyhow::Result<Model> {
		self.read_until_start().await?;

		let vertex_count = self.stream.read_u32().await? as usize;
		let mut vertices = Vec::with_capacity(vertex_count);
		for _ in 0..vertex_count {
			let pos_x = self.stream.read_f32().await?;
			let pos_y = self.stream.read_f32().await?;
			let pos_z = self.stream.read_f32().await?;

			let group_count = self.stream.read_u32().await? as usize;
			let mut groups = Vec::with_capacity(group_count);
			for _ in 0..group_count {
				let group_id = self.stream.read_u32().await? as usize;
				let weight = self.stream.read_f32().await?;
				groups.push((group_id, weight));
			}

			vertices.push(((pos_x, pos_y, pos_z), groups));
		}

		let polygon_count = self.stream.read_u32().await? as usize;
		for _ in 0..polygon_count {
			let normal_x = self.stream.read_f32().await?;
			let normal_y = self.stream.read_f32().await?;
			let normal_z = self.stream.read_f32().await?;

			let index_count = self.stream.read_u32().await? as usize;
			for _ in 0..index_count {
				let vertex_index = self.stream.read_u32().await? as usize;
			}

			let loop_idx_start = self.stream.read_u32().await? as usize;
			let loop_idx_end = self.stream.read_u32().await? as usize;
			let loop_range = loop_idx_start..loop_idx_end;
		}

		let loop_count = self.stream.read_u32().await? as usize;
		for idx in 0..loop_count {
			let vertex_index = self.stream.read_u32().await? as usize;
			let uv_x = self.stream.read_f32().await?;
			let uv_y = self.stream.read_f32().await?;
		}

		self.read_end().await?;

		self.into_model()
	}

	async fn read_until_start(&mut self) -> Result<(), ExportError> {
		while let Ok(byte) = self.stream.read_u8().await {
			if byte == 0b00 {
				return Ok(());
			}
		}
		Err(ExportError::StartMarkerMissing)
	}

	async fn read_end(&mut self) -> anyhow::Result<()> {
		let end_byte = self.stream.read_u8().await?;
		match end_byte == 0b00 {
			true => Ok(()),
			false => Err(ExportError::StopMarkerMissing)?,
		}
	}

	fn into_model(self) -> anyhow::Result<Model> {
		Ok(Model)
	}
}
