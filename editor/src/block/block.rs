use crystal_sphinx::block::Block;
use editor::asset::{BuildPath, TypeEditorMetadata};
use engine::{
	asset::{AnyBox, AssetResult},
	task::PinFutureResultLifetime,
};
use std::path::Path;

pub struct BlockEditorMetadata;
impl TypeEditorMetadata for BlockEditorMetadata {
	fn boxed() -> Box<dyn TypeEditorMetadata + 'static + Send + Sync> {
		Box::new(BlockEditorMetadata {})
	}

	fn read(&self, path: &Path, content: &str) -> AssetResult {
		editor::asset::deserialize::<Block>(&path, &content)
	}

	fn compile<'a>(
		&'a self,
		build_path: &'a BuildPath,
		asset: AnyBox,
	) -> PinFutureResultLifetime<'a, Vec<u8>> {
		Box::pin(async move { Ok(rmp_serde::to_vec(&asset.downcast::<Block>().unwrap())?) })
	}
}
