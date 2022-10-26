use super::Side;
use crate::graphics::voxel::Face;
use engine::asset::{self, AnyBox};
use enumset::EnumSet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextureEntry {
	pub texture_id: asset::Id,
	pub all_texture_ids: Vec<asset::Id>,
	pub biome_color: (bool, Option<asset::Id>),
}
impl TextureEntry {
	pub fn texture_ids(&self) -> &Vec<asset::Id> {
		&self.all_texture_ids
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
	asset_type: String,
	textures: Vec<(TextureEntry, EnumSet<Face>)>,
	/// True if the block's model is fully opaque/has no chance of seeing other blocks through it.
	is_opaque: bool,
}

impl Default for Block {
	fn default() -> Self {
		Self {
			asset_type: String::new(),
			textures: Vec::new(),
			is_opaque: true,
		}
	}
}

impl asset::Asset for Block {
	fn asset_type() -> asset::TypeId {
		"block"
	}

	fn decompile(bin: &Vec<u8>) -> anyhow::Result<AnyBox> {
		asset::decompile_asset::<Self>(bin)
	}
}

impl Block {
	pub fn is_opaque(&self) -> bool {
		self.is_opaque
	}

	fn set_is_opaque(&mut self, node: &kdl::KdlNode) {
		self.is_opaque = match node.get(0) {
			Some(entry) => match entry.value() {
				kdl::KdlValue::Bool(b) => *b,
				_ => false,
			},
			_ => true,
		};
	}

	pub fn textures(&self) -> &Vec<(TextureEntry, EnumSet<Face>)> {
		&self.textures
	}

	fn set_textures(&mut self, node: &kdl::KdlNode) {
		use engine::utility::kdl::{value_as_asset_id, value_map_asset_id};
		use std::convert::TryFrom;
		self.textures.clear();
		let mut found_faces = EnumSet::empty();

		fn parse_texture_node(node: &kdl::KdlNode) -> Option<TextureEntry> {
			let texture_id = match value_as_asset_id(&node, 0) {
				Some(id) => id,
				None => return None,
			};

			let mut entry = TextureEntry {
				all_texture_ids: vec![texture_id.clone()],
				texture_id,
				biome_color: (false, None),
			};

			if let Some(doc) = node.children() {
				for node in doc.nodes().iter() {
					match node.name().value() {
						"biome_color" => {
							entry.biome_color.0 = match node.get("enabled").map(|e| e.value()) {
								Some(kdl::KdlValue::Bool(b)) => *b,
								_ => false,
							};
							entry.biome_color.1 = match node.get("mask").map(|e| e.value()) {
								Some(kdl::KdlValue::String(v)) => value_map_asset_id(Some(&v)),
								_ => None,
							};
							if let Some(id) = &entry.biome_color.1 {
								entry.all_texture_ids.push(id.clone());
							}
						}
						_ => {}
					}
				}
			}

			Some(entry)
		}

		if let Some(doc) = node.children() {
			for node in doc.nodes().iter() {
				match node.name().value() {
					"sides" => {
						if let Some(doc) = node.children() {
							for texture_node in doc.nodes().iter() {
								if let Some(entry) = parse_texture_node(&texture_node) {
									if let Some(side) = Side::try_from(texture_node.name().value()).ok() {
										let faces = side.as_face_set();
										found_faces.insert_all(faces.clone());
										self.textures.push((entry, faces));
									}
								}
							}
						}
					}
					_ => {}
				}
			}
		}

		if let Some(entry) = parse_texture_node(&node) {
			self.textures.push((entry, found_faces.complement()));
		}
	}
}

impl engine::asset::kdl::Asset<Block> for Block {
	fn kdl_schema() -> kdl_schema::Schema<Block> {
		use kdl_schema::*;
		fn biome_color() -> Node<Block> {
			Node {
				name: Name::Defined("biome_color"),
				properties: vec![
					Property {
						name: "enabled",
						value: Value::Boolean,
						optional: false,
					},
					Property {
						name: "mask",
						value: Value::String(None),
						optional: true,
					},
				],
				..Default::default()
			}
		}
		fn texture_node(name: &'static str) -> Node<Block> {
			Node {
				name: Name::Defined(name),
				values: Items::Select(vec![Value::String(None)]),
				children: Items::Select(vec![biome_color()]),
				..Default::default()
			}
		}
		fn texture_sides() -> Node<Block> {
			Node {
				name: Name::Defined("sides"),
				children: Items::Select(
					Side::all()
						.into_iter()
						.map(|side| texture_node(side.as_str()))
						.collect(),
				),
				..Default::default()
			}
		}
		Schema {
			nodes: Items::Ordered(vec![
				asset::kdl::asset_type::schema::<Block>(|asset, node| {
					asset.asset_type = asset::kdl::asset_type::get(node);
				}),
				Node {
					name: Name::Defined("is_opaque"),
					values: Items::Ordered(vec![Value::Boolean]),
					on_validation_successful: Some(Block::set_is_opaque),
					..Default::default()
				},
				Node {
					children: Items::Select(vec![biome_color(), texture_sides()]),
					on_validation_successful: Some(Block::set_textures),
					..texture_node("textures")
				},
			]),
			..Default::default()
		}
	}
}
