use crate::graphics::model::Model as ModelTrait;
use engine::{
	graphics::{
		flags, pipeline,
		types::{Vec2, Vec3},
		vertex_object,
	},
	math::nalgebra::{Vector2, Vector3},
};
use serde::{Deserialize, Serialize};

mod cache;
pub use cache::*;

/// Model data representing an exported Blender file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
	pub vertices: Vec<Vertex>,
	// each value refers to an entry in vertices
	pub indices: Vec<u32>,
	// length matches vertices
	// contains the weight of each group for a given vertex
	pub vertex_weights: Vec<Vec<VertexWeight>>,
}

impl ModelTrait for Model {
	type Vertex = Vertex;
	type Index = u32;

	fn vertices(&self) -> &Vec<Self::Vertex> {
		&self.vertices
	}

	fn indices(&self) -> &Vec<Self::Index> {
		&self.indices
	}
}

/// Vertex data of an exported Blender file.
/// Partially composed with polygon face data.
///
/// NOTE: This composition will likely cause the blender model compiled binary to be larger than it needs to be.
/// While the asset compilation does ignore duplicate entries, it also create duplicate vertices which have different
/// normal and uv data. There is room for optimation here where we only convert to engine-specific structures at runtime.
/// This is also noted in `EDITOR/src/blender_model/exporter/blender_data.rs/BlenderData::process()`.
#[vertex_object]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Vertex {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub position: Vec3,
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub normal: Vec3,
	#[vertex_attribute([R, G], Bit32, SFloat)]
	pub tex_coord: Vec2,
}

/// The group-weight association for a given vertex of an exported Blender file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VertexWeight {
	pub group_index: usize,
	pub weight: f32,
}
