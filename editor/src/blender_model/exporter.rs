use engine::task::JoinHandle;
use futures_util::future::Future;
use std::path::PathBuf;

static EXPORT_SCRIPT_PATH: &'static str = "./scripts/export_blender_model.py";
static EXPORT_SCRIPT: &'static str = std::include_str!("exporter/script.py");

mod builder;
pub use builder::*;

mod blender_data;
pub use blender_data::*;

mod error;
pub use error::*;

/// Temporary editor-only empty struct.
/// This needs to be moved to engine/game layer when I am ready to embed data in the BlenderModel asset
pub struct Model;

fn create_script_path() -> anyhow::Result<PathBuf> {
	let cwd = std::env::current_dir()?;
	let mut path = cwd.clone();
	path.push(EXPORT_SCRIPT_PATH);
	path.canonicalize()?;
	Ok(path)
}

async fn ensure_export_script() -> anyhow::Result<PathBuf> {
	use tokio::fs;
	let script_path = create_script_path()?;
	fs::create_dir_all(script_path.parent().unwrap()).await?;
	fs::write(&script_path, EXPORT_SCRIPT).await?;
	Ok(script_path)
}
