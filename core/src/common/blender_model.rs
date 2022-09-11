use engine::{
	asset::{self, kdl, AnyBox, Asset, TypeId},
	math::nalgebra::{Vector2, Vector3},
};
use serde::{Deserialize, Serialize};

// TODO: This isn't common between client and server, its only for the client.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BlenderModel {
	asset_type: String,
	mesh_name: String,
	compiled: Option<Model>,
}

/// Model data representing an exported Blender file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
	pub vertices: Vec<Vertex>,
	// each value refers to an entry in vertices
	pub indices: Vec<usize>,
	// length matches vertices
	// contains the weight of each group for a given vertex
	pub vertex_weights: Vec<Vec<VertexWeight>>,
}

/// Vertex data of an exported Blender file.
/// Partially composed with polygon face data.
///
/// NOTE: This composition will likely cause the blender model compiled binary to be larger than it needs to be.
/// While the asset compilation does ignore duplicate entries, it also create duplicate vertices which have different
/// normal and uv data. There is room for optimation here where we only convert to engine-specific structures at runtime.
/// This is also noted in `EDITOR/src/blender_model/exporter/blender_data.rs/BlenderData::process()`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Vertex {
	pub position: Vector3<f32>,
	pub normal: Vector3<f32>,
	pub tex_coord: Vector2<f32>,
}

/// The group-weight association for a given vertex of an exported Blender file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VertexWeight {
	pub group_index: usize,
	pub weight: f32,
}

impl Asset for BlenderModel {
	fn asset_type() -> TypeId {
		"blender-model"
	}

	fn decompile(bin: &Vec<u8>) -> anyhow::Result<AnyBox> {
		asset::decompile_asset::<Self>(bin)
	}
}

impl BlenderModel {
	#[doc(hidden)]
	pub fn set_compiled(&mut self, model: Model) {
		self.compiled = Some(model)
	}

	pub fn compiled(&self) -> &Model {
		&self.compiled.as_ref().unwrap()
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
