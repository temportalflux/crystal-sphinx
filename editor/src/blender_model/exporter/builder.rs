use crate::blender_model::exporter::{ensure_export_script, BlenderData, ExportError};
use anyhow::Context;
use crystal_sphinx::common::blender_model::Model;
use std::path::PathBuf;

pub struct Builder {
	blend_path: PathBuf,
}

impl Builder {
	pub fn new() -> Self {
		Self {
			blend_path: PathBuf::new(),
		}
	}

	pub fn with_blend(mut self, path: PathBuf) -> Self {
		self.blend_path = path;
		self
	}

	pub async fn build(self) -> anyhow::Result<Model> {
		use tokio::{io::*, process::*};
		let script_path = ensure_export_script()
			.await
			.context("failed to ensure export script")?;
		let mut exporter = Command::new("blender")
			.arg(self.blend_path.to_str().unwrap())
			.arg("--background")
			.arg("--python")
			.arg(script_path.to_str().unwrap())
			.arg("--")
			.arg("--mesh_name")
			.arg("Model")
			.arg("--output_mode")
			.arg("BYTES")
			.stdin(std::process::Stdio::null())
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped())
			.spawn()
			.context("failed to spawn exporter")?;
		let out_stream = exporter.stdout.take().unwrap();
		let mut err_stream = exporter.stderr.take().unwrap();
		// Run the exporter until completion in a detached task so that
		// it will run in parallel to the processing of its output.
		let exporter_task = tokio::task::spawn(async move { exporter.wait().await });
		// Create a detached task which runs in parallel to the above script process and the below error processing.
		// This task parses the output stream of the export script to turn it into a struct the asset will accept.
		// It is detached so that it can run while the current scope waits for the err stream to be complete.
		// This way, the output is processed immediately as the script writes it.
		let read_exported_data =
			tokio::task::spawn(async move { BlenderData::new(out_stream).process().await });
		// Read until the EOF descriptor in the err stream.
		// This will only occur once the process basically closes the stream
		// (but that doesn't mean the process task has actually finished yet).
		let errors = {
			let mut buffer = String::new();
			err_stream.read_to_string(&mut buffer).await?;
			buffer
		};
		match errors.is_empty() {
			// Horray! No errors!
			true => {
				// Ensure the detached exporter process has finished.
				let _status = exporter_task.await?;
				// Join the exporter_data task and resolve any errors it had while reading data.
				Ok(read_exported_data.await??)
			}
			// There was an error output to the err stream.
			// We need to parse the error string to return the proper error.
			false => {
				use std::str::FromStr;
				// If we encounter errors, we halt the processing of exported data (its no longer relevant).
				read_exported_data.abort();
				// Wait for the detached exporter process to finish before we return the errors.
				let _status = exporter_task.await?;
				// Parse the error string as a rust error
				Err(ExportError::from_str(&errors)?)?
			}
		}
	}
}
