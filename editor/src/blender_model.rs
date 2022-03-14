use anyhow::Result;
use crystal_sphinx::common::BlenderModel;
use editor::asset::{BuildPath, EditorOps};
use engine::{
	asset::AnyBox,
	task::{JoinHandle, PinFutureResult},
};
use std::path::PathBuf;

pub mod exporter;

pub struct BlenderModelEditorOps;
impl EditorOps for BlenderModelEditorOps {
	type Asset = BlenderModel;

	fn get_related_paths(mut path: PathBuf) -> PinFutureResult<Option<Vec<PathBuf>>> {
		path.set_extension("blend");
		Box::pin(async move { Ok(Some(vec![path])) })
	}

	fn read(source: PathBuf, file_content: String) -> PinFutureResult<AnyBox> {
		Box::pin(async move { editor::asset::deserialize::<BlenderModel>(&source, &file_content) })
	}

	fn compile(build_path: BuildPath, asset: AnyBox) -> PinFutureResult<Vec<u8>> {
		Box::pin(async move {
			use tokio::{fs, io::*, process::*};

			let mut model = asset.downcast::<BlenderModel>().unwrap();

			log::debug!("processing blender model: {}", build_path.source.display());

			let exported_data = exporter::Builder::new()
				.with_blend(build_path.source_with_ext("blend"))
				.build()
				.await?;

			// Temporily force the operation to "fail" so the binary is not created
			return Err(exporter::FailedToParseExportError::Unknown)?;
			//Ok(rmp_serde::to_vec(&model)?)
		})
	}
}
