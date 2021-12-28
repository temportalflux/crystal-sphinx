use super::Side;
use engine::asset::{self, AssetResult, TypeMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Block {
	asset_type: String,
	textures: HashMap<Side, asset::Id>,
}

impl asset::Asset for Block {
	fn metadata() -> Box<dyn TypeMetadata> {
		Box::new(BlockMetadata {})
	}
}

impl Block {
	pub fn textures(&self) -> &HashMap<Side, asset::Id> {
		&self.textures
	}

	fn set_textures(&mut self, node: &kdl::KdlNode) {
		use engine::utility::kdl::value_as_asset_id;
		use std::convert::TryFrom;
		self.textures.clear();
		for texture_node in node.children.iter() {
			let side_opt = Side::try_from(texture_node.name.as_str()).ok();
			let id_opt = value_as_asset_id(texture_node, 0);
			if let Some((side, asset_id)) = side_opt.zip(id_opt) {
				for side in side.as_side_list().into_iter() {
					self.textures.insert(side, asset_id.clone());
				}
			}
		}
		if let Some(default_texture) = value_as_asset_id(node, 0) {
			for side in Side::all_real().iter() {
				if !self.textures.contains_key(&side) {
					self.textures.insert(*side, default_texture.clone());
				}
			}
		}
	}
}

impl engine::asset::kdl::Asset<Block> for Block {
	fn kdl_schema() -> kdl_schema::Schema<Block> {
		use kdl_schema::*;
		fn sided_texture(name: &'static str) -> Node<Block> {
			Node {
				name: Name::Defined(name),
				values: Items::Ordered(vec![Value::String(None)]),
				..Default::default()
			}
		}
		Schema {
			nodes: Items::Ordered(vec![
				asset::kdl::asset_type::schema::<Block>(|asset, node| {
					asset.asset_type = asset::kdl::asset_type::get(node);
				}),
				Node {
					name: Name::Defined("textures"),
					values: Items::Select(vec![Value::String(None)]),
					children: Items::Select(
						Side::all()
							.into_iter()
							.map(|side| sided_texture(side.as_str()))
							.collect(),
					),
					on_validation_successful: Some(Block::set_textures),
					..Default::default()
				},
			]),
			..Default::default()
		}
	}
}

/// The metadata about the [`Block`] asset type.
pub struct BlockMetadata {}

impl TypeMetadata for BlockMetadata {
	fn name(&self) -> asset::TypeId {
		"block"
	}

	fn decompile(&self, bin: &Vec<u8>) -> AssetResult {
		asset::decompile_asset::<Block>(bin)
	}
}
