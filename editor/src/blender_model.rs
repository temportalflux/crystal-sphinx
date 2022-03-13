use anyhow::Result;
use crystal_sphinx::common::BlenderModel;
use editor::asset::{BuildPath, TypeEditorMetadata};
use engine::{
	asset::{AnyBox, AssetResult},
	task::PinFutureResultLifetime,
};
use std::{collections::HashMap, path::Path};

static EXPORT_SCRIPT_PATH: &'static str = "./scripts/blender_model.py";
static EXPORT_SCRIPT: &'static str = std::include_str!("blender_model.py");

pub struct BlenderModelEditorMetadata;
impl TypeEditorMetadata for BlenderModelEditorMetadata {
	fn boxed() -> Box<dyn TypeEditorMetadata + 'static + Send + Sync> {
		Box::new(BlenderModelEditorMetadata {})
	}

	fn read(&self, path: &Path, content: &str) -> AssetResult {
		editor::asset::deserialize::<BlenderModel>(&path, &content)
	}

	fn compile<'a>(
		&'a self,
		build_path: &'a BuildPath,
		asset: AnyBox,
	) -> PinFutureResultLifetime<'a, Vec<u8>> {
		Box::pin(async move {
			let mut model = asset.downcast::<BlenderModel>().unwrap();

			use std::process::*;

			let cwd = std::env::current_dir()?;
			let script_path = {
				let mut path = cwd.clone();
				path.push(EXPORT_SCRIPT_PATH);
				path.canonicalize()?
			};
			std::fs::create_dir_all(script_path.parent().unwrap())?;
			std::fs::write(&script_path, EXPORT_SCRIPT)?;

			log::debug!("processing blender model: {}", build_path.source.display());
			let blend_path = build_path.source_with_ext("blend");

			let output = Command::new("blender")
				.arg(blend_path.to_str().unwrap())
				.arg("--background")
				.arg("--python")
				.arg(script_path.to_str().unwrap())
				.arg("--")
				.arg("--mesh_name")
				.arg("Model")
				.arg("--output_mode")
				.arg("BYTES")
				.output()?;

			let errors = String::from_utf8(output.stderr)?;
			if !errors.is_empty() {
				use std::str::FromStr;
				return Err(ExportError::from_str(&errors)?)?;
			}

			let mut stream = ExportStream::new(output.stdout);

			while let Ok(byte) = stream.next_byte() {
				if byte == 0b00 {
					// Found start of data stream
					break;
				}
			}

			let vertex_count = stream.next_num::<u32>()? as usize;
			let mut vertices = Vec::with_capacity(vertex_count);
			for _ in 0..vertex_count {
				let pos_x = stream.next_num::<f32>()?;
				let pos_y = stream.next_num::<f32>()?;
				let pos_z = stream.next_num::<f32>()?;

				let group_count = stream.next_num::<u32>()? as usize;
				let mut groups = Vec::with_capacity(group_count);
				for _ in 0..group_count {
					let group_id = stream.next_num::<u32>()? as usize;
					let weight = stream.next_num::<f32>()?;
					groups.push((group_id, weight));
				}

				vertices.push(((pos_x, pos_y, pos_z), groups));
			}

			let polygon_count = stream.next_num::<u32>()? as usize;
			for _ in 0..polygon_count {
				let normal_x = stream.next_num::<f32>()?;
				let normal_y = stream.next_num::<f32>()?;
				let normal_z = stream.next_num::<f32>()?;

				let index_count = stream.next_num::<u32>()? as usize;
				for _ in 0..index_count {
					let vertex_index = stream.next_num::<u32>()? as usize;
				}

				let loop_idx_start = stream.next_num::<u32>()? as usize;
				let loop_idx_end = stream.next_num::<u32>()? as usize;
				let loop_range = loop_idx_start..loop_idx_end;
			}

			let loop_count = stream.next_num::<u32>()? as usize;
			for idx in 0..loop_count {
				let vertex_index = stream.next_num::<u32>()? as usize;
				let uv_x = stream.next_num::<f32>()?;
				let uv_y = stream.next_num::<f32>()?;
			}

			assert_eq!(stream.next_byte().ok(), Some(0b00));

			// Temporily force the operation to "fail" so the binary is not created
			return Err(FailedToParseExportError::Unknown)?;
			//Ok(rmp_serde::to_vec(&model)?)
		})
	}
}

struct ExportStream(bytes::Bytes);
impl ExportStream {
	fn new(data: Vec<u8>) -> Self {
		Self(bytes::Bytes::from(data))
	}

	fn next_byte(&mut self) -> Result<u8> {
		if self.0.len() < 1 {
			return Err(StreamError::ReachedEndOfStream)?;
		}
		Ok(self.0.split_to(1).into_iter().next().unwrap())
	}

	fn next_num<T>(&mut self) -> Result<T>
	where
		T: serde::de::DeserializeOwned + Sized + Send + Sync + 'static,
	{
		let byte_count = std::mem::size_of::<T>();
		if self.0.len() < byte_count {
			return Err(StreamError::ReachedEndOfStream)?;
		}
		let encoded = self.0.split_to(byte_count);
		Ok(bincode::deserialize(&encoded)?)
	}
}

#[derive(thiserror::Error, Debug)]
enum StreamError {
	#[error("Reached end of stream")]
	ReachedEndOfStream,
}

#[derive(thiserror::Error, Debug)]
enum ExportError {
	#[error("Blender export script was not provided an argument named \"{0}\".")]
	MissingArgument(String),
	#[error("Model mesh is not triangulated. Only triangle polygons are currently supported.")]
	NGonsNotSupported,
	#[error("Mesh object named \"{0}\" does not exist.")]
	MeshDoesNotExist(/*mesh_object_name*/ String),
	#[error(
		"Mesh object named \"{0}\" was found, but has the object type \"{1}\" instead of MESH."
	)]
	MeshHasWrongType(
		/*mesh_object_name*/ String,
		/*object_type*/ String,
	),
	#[error("Mesh object \"{0}\" has a parent \"{1}\" whose type is \"{2}\" and not ARMATURE.")]
	MeshParentIsNotNoneOrArmature(
		/*mesh_object_name*/ String,
		/*parent_name*/ String,
		/*parent_object_type*/ String,
	),
	#[error(
		"Mesh object \"{0}\" is somehow missing data for its mesh \"{1}\", is the file corrupt?"
	)]
	MeshDataMissing(/*mesh_object_name*/ String, /*mesh_name*/ String),
}
impl std::str::FromStr for ExportError {
	type Err = FailedToParseExportError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		use regex::*;
		use FailedToParseExportError::*;
		let re = Regex::new(r"^(?P<error>[^\(]+)(?P<args>.*)$").unwrap();
		let groups = re
			.captures(s.trim())
			.ok_or(InvalidErrorFormat(s.to_owned(), re.as_str().to_owned()))?;
		let error_name = groups.name("error").unwrap().as_str();
		match error_name {
			"MissingArgument" => {
				let args = ArgMatch::new(s, &["arg_name"]).parse(groups.name("args"))?;
				Ok(Self::MissingArgument(args["arg_name"].clone()))
			}
			"NGonsNotSupported" => Ok(Self::NGonsNotSupported),
			"MeshDoesNotExist" => {
				let args = ArgMatch::new(s, &["arg_name"]).parse(groups.name("args"))?;
				Ok(Self::MeshDoesNotExist(args["arg_name"].clone()))
			}
			"MeshHasWrongType" => {
				let args =
					ArgMatch::new(s, &["obj_name", "obj_type"]).parse(groups.name("args"))?;
				Ok(Self::MeshHasWrongType(
					args["obj_name"].clone(),
					args["obj_type"].clone(),
				))
			}
			"MeshParentIsNotNoneOrArmature" => {
				let args = ArgMatch::new(s, &["obj_name", "parent_name", "parent_type"])
					.parse(groups.name("args"))?;
				Ok(Self::MeshParentIsNotNoneOrArmature(
					args["obj_name"].clone(),
					args["parent_name"].clone(),
					args["parent_type"].clone(),
				))
			}
			"MeshDataMissing" => {
				let args =
					ArgMatch::new(s, &["obj_name", "mesh_name"]).parse(groups.name("args"))?;
				Ok(Self::MeshDataMissing(
					args["obj_name"].clone(),
					args["mesh_name"].clone(),
				))
			}
			_ => Err(Unknown),
		}
	}
}

struct ArgMatch {
	error_name: String,
	arg_names: Vec<&'static str>,
	pattern: regex::Regex,
	args: HashMap<&'static str, String>,
}
impl ArgMatch {
	fn new(error_name: &str, names: &[&'static str]) -> Self {
		let pattern = regex::Regex::new(&{
			let args = names.iter().fold(String::new(), |mut pattern, &name| {
				if !pattern.is_empty() {
					pattern.push_str(", ");
				}
				pattern.push_str(&format!("(?P<{}>.+)", name));
				pattern
			});
			format!(r"^\({}\)$", args)
		})
		.unwrap();
		Self {
			error_name: error_name.to_owned(),
			arg_names: names.to_vec(),
			pattern,
			args: HashMap::new(),
		}
	}
	fn parse(mut self, group: Option<regex::Match>) -> Result<Self, FailedToParseExportError> {
		let args = match group {
			Some(args) => args,
			None => {
				return Err(FailedToParseExportError::ErrorMissingArgs(
					self.error_name.clone(),
				))
			}
		};
		let capture = self.pattern.captures(args.as_str()).ok_or(
			FailedToParseExportError::InvalidErrorFormat(
				self.error_name.clone(),
				self.pattern.as_str().to_owned(),
			),
		)?;
		for &arg_name in self.arg_names.iter() {
			let value =
				capture
					.name(arg_name)
					.ok_or(FailedToParseExportError::ArgumentNotProvided(
						self.error_name.clone(),
						arg_name,
					))?;
			self.args.insert(arg_name, value.as_str().to_owned());
		}
		Ok(self)
	}
}
impl std::ops::Index<&str> for ArgMatch {
	type Output = String;
	fn index(&self, arg_name: &str) -> &Self::Output {
		self.args.get(arg_name).unwrap()
	}
}

#[derive(thiserror::Error, Debug)]
enum FailedToParseExportError {
	#[error("Failed to parse error \"{0}\" with format \"{1}\".")]
	InvalidErrorFormat(String, String),
	#[error("Received an unknown error from the blender export script.")]
	Unknown,
	#[error("Received error \"{0}\" from script, but no arguments were provided.")]
	ErrorMissingArgs(String),
	#[error("Received error \"{0}\" from script, but argument \"{1}\" was not provided.")]
	ArgumentNotProvided(String, &'static str),
}
