use anyhow::Context;
use crystal_sphinx::client::model::blender;
use editor::asset::{BuildPath, EditorOps};
use engine::{asset::AnyBox, task::PinFutureResult};
use std::path::PathBuf;

pub mod exporter;

mod polygon;
pub use polygon::*;
mod point;
pub use point::*;

pub struct BlenderModelEditorOps;
impl EditorOps for BlenderModelEditorOps {
	type Asset = blender::Asset;

	fn get_related_paths(mut path: PathBuf) -> PinFutureResult<Option<Vec<PathBuf>>> {
		path.set_extension("blend");
		Box::pin(async move { Ok(Some(vec![path])) })
	}

	fn read(source: PathBuf, file_content: String) -> PinFutureResult<AnyBox> {
		Box::pin(
			async move { editor::asset::deserialize::<blender::Asset>(&source, &file_content) },
		)
	}

	fn compile(build_path: BuildPath, asset: AnyBox) -> PinFutureResult<Vec<u8>> {
		Box::pin(async move {
			let mut model = asset.downcast::<blender::Asset>().unwrap();

			let exported_data = exporter::Builder::new()
				.with_blend(build_path.source_with_ext("blend"))
				.build()
				.await
				.context("exporting blender file")?;

			model.set_compiled(exported_data);

			Ok(rmp_serde::to_vec(&model)?)
		})
	}
}
