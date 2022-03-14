use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum ExportError {
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
pub enum FailedToParseExportError {
	#[error("Failed to parse error \"{0}\" with format \"{1}\".")]
	InvalidErrorFormat(String, String),
	#[error("Received an unknown error from the blender export script.")]
	Unknown,
	#[error("Received error \"{0}\" from script, but no arguments were provided.")]
	ErrorMissingArgs(String),
	#[error("Received error \"{0}\" from script, but argument \"{1}\" was not provided.")]
	ArgumentNotProvided(String, &'static str),
}
