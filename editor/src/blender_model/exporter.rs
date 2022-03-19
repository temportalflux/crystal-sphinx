use anyhow::Context;
use std::path::PathBuf;

static EXPORT_SCRIPT: &'static str = std::include_str!("exporter/script.py");

mod builder;
pub use builder::*;

mod blender_data;
pub use blender_data::*;

mod error;
use engine::math::nalgebra::{Vector2, Vector3};
pub use error::*;

/// Temporary editor-only empty struct.
/// This needs to be moved to engine/game layer when I am ready to embed data in the BlenderModel asset
#[derive(Debug)]
pub struct Model {
	pub vertices: Vec<Vertex>,
	// each value refers to an entry in vertices
	pub indices: Vec<usize>,
	// length matches vertices
	// contains the weight of each group for a given vertex
	pub vertex_weights: Vec<Vec<VertexWeight>>,
}
#[derive(Debug, PartialEq)]
pub struct Vertex {
	pub position: Vector3<f32>,
	pub normal: Vector3<f32>,
	pub tex_coord: Vector2<f32>,
}
#[derive(Debug)]
pub struct VertexWeight {
	pub group_index: usize,
	pub weight: f32,
}

fn create_script_path() -> anyhow::Result<PathBuf> {
	let cwd = std::env::current_dir()?;
	let mut path = cwd.clone();
	path.push("scripts");
	path.push("export_blender_model.py");
	Ok(path)
}

async fn ensure_export_script() -> anyhow::Result<PathBuf> {
	use tokio::fs;
	let script_path = create_script_path().context("failed to create script path")?;
	fs::create_dir_all(script_path.parent().unwrap())
		.await
		.context("failed to ensure scripts directory")?;
	fs::write(&script_path, EXPORT_SCRIPT)
		.await
		.context("failed to write export script to disk")?;
	Ok(script_path)
}
