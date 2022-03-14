use engine::asset::{self, kdl, AnyBox, Asset, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BlenderModel {
	asset_type: String,
	mesh_name: String,
}

impl Asset for BlenderModel {
	fn asset_type() -> TypeId {
		"blender-model"
	}

	fn decompile(bin: &Vec<u8>) -> anyhow::Result<AnyBox> {
		asset::decompile_asset::<Self>(bin)
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
