use crystal_sphinx::block::Block;
use editor::asset::TypeEditorMetadata;
use engine::{
	asset::{AnyBox, AssetResult},
	utility::Result,
};
use std::path::Path;

pub struct BlockEditorMetadata;
impl TypeEditorMetadata for BlockEditorMetadata {
	fn boxed() -> Box<dyn TypeEditorMetadata> {
		Box::new(BlockEditorMetadata {})
	}

	fn read(&self, path: &Path, content: &str) -> AssetResult {
		editor::asset::deserialize::<Block>(&path, &content)
	}

	fn compile(&self, _: &Path, asset: AnyBox) -> Result<Vec<u8>> {
		Ok(rmp_serde::to_vec(&asset.downcast::<Block>().unwrap())?)
	}
}
