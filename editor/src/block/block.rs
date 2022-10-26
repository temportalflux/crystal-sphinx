use crystal_sphinx::block::Block;
use editor::asset::{BuildPath, EditorOps};
use engine::{asset::AnyBox, task::PinFutureResult};

pub struct BlockEditorOps;
impl EditorOps for BlockEditorOps {
	type Asset = Block;

	fn read(source: std::path::PathBuf, file_content: String) -> PinFutureResult<AnyBox> {
		Box::pin(async move { editor::asset::deserialize::<Block>(&source, &file_content) })
	}

	fn compile(_build_path: BuildPath, asset: AnyBox) -> PinFutureResult<Vec<u8>> {
		Box::pin(async move {
			Ok(rmp_serde::to_vec_named(
				&asset.downcast::<Block>().unwrap(),
			)?)
		})
	}
}
