use anyhow::Context;
use std::path::PathBuf;

static EXPORT_SCRIPT: &'static str = std::include_str!("exporter/script.py");

mod builder;
pub use builder::*;

mod blender_data;
pub use blender_data::*;

mod error;
pub use error::*;

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
