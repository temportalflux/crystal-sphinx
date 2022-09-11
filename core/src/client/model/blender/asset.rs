use super::model::Model;
use engine::asset::{self, kdl, AnyBox, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Asset {
	asset_type: String,
	mesh_name: String,
	compiled: Option<Model>,
}

impl asset::Asset for Asset {
	fn asset_type() -> TypeId {
		"blender-model"
	}

	fn decompile(bin: &Vec<u8>) -> anyhow::Result<AnyBox> {
		asset::decompile_asset::<Self>(bin)
	}
}

impl Asset {
	#[doc(hidden)]
	pub fn set_compiled(&mut self, model: Model) {
		self.compiled = Some(model)
	}

	pub fn compiled(&self) -> &Model {
		&self.compiled.as_ref().unwrap()
	}
}

impl kdl::Asset<Asset> for Asset {
	fn kdl_schema() -> kdl_schema::Schema<Asset> {
		use kdl_schema::*;
		Schema {
			nodes: Items::Ordered(vec![
				kdl::asset_type::schema::<Asset>(|asset, node| {
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
