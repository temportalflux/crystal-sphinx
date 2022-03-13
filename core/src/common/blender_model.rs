use engine::asset::{self, kdl, Asset, AssetResult, TypeId, TypeMetadata};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BlenderModel {
	asset_type: String,
	mesh_name: String,
}

impl Asset for BlenderModel {
	fn metadata() -> Box<dyn TypeMetadata> {
		Box::new(Metadata {})
	}
}

/// The metadata about the [`BlenderModel`] asset type.
struct Metadata {}

impl TypeMetadata for Metadata {
	fn name(&self) -> TypeId {
		"blender-model"
	}

	fn decompile(&self, bin: &Vec<u8>) -> AssetResult {
		asset::decompile_asset::<BlenderModel>(bin)
	}
}

impl kdl::Asset<BlenderModel> for BlenderModel {
	fn kdl_schema() -> kdl_schema::Schema<BlenderModel> {
		use kdl_schema::*;
		Schema {
			nodes: Items::Ordered(vec![
				kdl::asset_type::schema::<BlenderModel>(|asset, node| {
					asset.asset_type = kdl::asset_type::get(node);
				}),
				Node {
					name: Name::Defined("mesh-name"),
					values: Items::Ordered(vec![Value::String(None)]),
					on_validation_successful: Some(|model, node| {
						model.mesh_name = utility::value_as_string(&node, 0).unwrap().clone();
					}),
					..Default::default()
				},
			]),
			..Default::default()
		}
	}
}
