use anyhow::Result;
use crystal_sphinx::common::BlenderModel;
use editor::asset::TypeEditorMetadata;
use engine::asset::{AnyBox, AssetResult};
use std::path::Path;

static EXPORT_SCRIPT_PATH: &'static str = "./scripts/blender_model.py";
static EXPORT_SCRIPT: &'static str = std::include_str!("blender_model.py");

pub struct BlenderModelEditorMetadata;
impl TypeEditorMetadata for BlenderModelEditorMetadata {
	fn boxed() -> Box<dyn TypeEditorMetadata> {
		Box::new(BlenderModelEditorMetadata {})
	}

	fn read(&self, path: &Path, content: &str) -> AssetResult {
		editor::asset::deserialize::<BlenderModel>(&path, &content)
	}

	fn process_intermediate(&self, json_path: &Path, relative_path: &Path, asset: &mut AnyBox) -> Result<()> {
		use std::process::*;
		use std::io::Write;
		
		let model = asset.downcast_ref::<BlenderModel>().unwrap();

		let cwd = std::env::current_dir()?;
		let script_path = {
			let mut path = cwd.clone();
			path.push(EXPORT_SCRIPT_PATH);
			path.canonicalize()?
		};
		std::fs::create_dir_all(script_path.parent().unwrap())?;
		std::fs::write(&script_path, EXPORT_SCRIPT)?;

		log::debug!("processing blender model: {}", json_path.display());
		let blend_path = {
			let mut path = json_path.parent().unwrap().to_owned();
			path.push(json_path.file_stem().unwrap());
			path.set_extension("blend");
			path
		};

		let output = Command::new("blender")
			.arg(blend_path.to_str().unwrap())
			.arg("--background")
			.arg("--python")
			.arg(script_path.to_str().unwrap())
			.arg("--")
			.arg("--mesh_name")
			.arg("Model")
			.arg("--output_path")
			.arg("mlem")
			.output()?;
		let _ = std::io::stdout().write_all(&output.stdout);
		unimplemented!();
		Ok(())
	}

	fn compile(&self, path: &Path, asset: AnyBox) -> Result<Vec<u8>> {
		let mut model = asset.downcast::<BlenderModel>().unwrap();

		Ok(rmp_serde::to_vec(&model)?)
	}
}
