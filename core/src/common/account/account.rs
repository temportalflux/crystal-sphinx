use crate::common::{
	account::key::{self, Certificate, Key, PrivateKey, PublicKey},
	utility::DataFile,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Account {
	root: PathBuf,
	display_name: String,
	key: Key,
}

impl std::fmt::Display for Account {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Account({})", self.id())
	}
}

impl Account {
	/// Creates a new account, complete with a generated RSA key.
	pub fn new_private(parent_dir: &Path, display_name: String) -> Result<Self> {
		let (id, certificate, private_key) = key::create_pem()?;

		let mut root = parent_dir.to_owned();
		root.push(id.to_string());

		if !root.exists() {
			std::fs::create_dir_all(&root)?;
		}

		std::fs::write(&Certificate::make_path(&root), certificate)?;
		std::fs::write(&PrivateKey::make_path(&root), private_key)?;

		let certificate = Certificate::load(&root)?;
		let private_key = PrivateKey::load(&root)?;
		assert_eq!(id, certificate.fingerprint());
		let key = Key::Private(certificate, private_key);

		Ok(Self {
			root,
			display_name,
			key,
		})
	}

	pub fn new_public(parent_dir: &Path, id: String, public_key: PublicKey) -> Self {
		let mut root = parent_dir.to_owned();
		root.push(id.to_string());
		Self {
			root,
			display_name: "unknown".to_owned(),
			key: Key::Public(public_key),
		}
	}

	pub fn path(&self) -> &Path {
		&self.root
	}

	pub fn id(&self) -> String {
		self.root.file_name().unwrap().to_str().unwrap().to_string()
	}

	pub fn set_display_name(&mut self, name: String) {
		self.display_name = name;
	}

	pub fn display_name(&self) -> &String {
		&self.display_name
	}

	pub fn key(&self) -> &Key {
		&self.key
	}
}

impl DataFile for Account {
	fn file_name() -> &'static str {
		"meta.kdl"
	}

	fn save_to(&self, file_path: &Path) -> Result<()> {
		let root = file_path.parent().unwrap().clone();
		let key_id = match &self.key {
			Key::Private(certificate, private_key) => {
				certificate.save(&root)?;
				private_key.save(&root)?;
				"Private"
			}
			Key::Public(public_key) => {
				public_key.save(&root)?;
				"Public"
			}
		};

		let mut text = String::new();
		text += &format!("display-name \"{}\"\n", self.display_name);
		text += &format!("key \"{}\"\n", key_id);
		std::fs::write(&file_path, text)?;

		Ok(())
	}

	fn load_from(file_path: &Path) -> Result<Self> {
		let root = file_path.parent().unwrap().to_owned();
		let meta_text = std::fs::read_to_string(&file_path)?;
		let nodes = kdl::parse_document(meta_text)?;
		let mut display_name = String::new();
		let mut key_id = String::new();
		for node in nodes.into_iter() {
			match node.name.as_str() {
				"display-name" => {
					let value = node
						.values
						.first()
						.ok_or(LoadError::MissingValue("display-name", 0))?;
					match value {
						kdl::KdlValue::String(s) => display_name = s.clone(),
						_ => return Err(LoadError::InvalidType("display-name", 0, "String"))?,
					}
				}
				"key" => {
					let value = node
						.values
						.first()
						.ok_or(LoadError::MissingValue("key", 0))?;
					match value {
						kdl::KdlValue::String(s) => key_id = s.clone(),
						_ => return Err(LoadError::InvalidType("key", 0, "String"))?,
					}
				}
				_ => {}
			}
		}
		let key = match key_id.as_str() {
			"Private" => {
				let certificate = Certificate::load(&root)?;
				let private_key = PrivateKey::load(&root)?;
				Key::Private(certificate, private_key)
			}
			"Public" => {
				let public_key = PublicKey::load(&root)?;
				Key::Public(public_key)
			}
			_ => return Err(LoadError::MissingNode("key"))?,
		};
		if display_name.is_empty() {
			return Err(LoadError::MissingNode("display-name"))?;
		}
		Ok(Self {
			root,
			display_name,
			key,
		})
	}
}

#[derive(thiserror::Error, Debug)]
enum LoadError {
	#[error("missing node named {0}")]
	MissingNode(&'static str),
	#[error("missing kdl value for {0} at index {1}")]
	MissingValue(&'static str, usize),
	#[error("kdl value in node {0} index {1} is not a {2}")]
	InvalidType(&'static str, usize, &'static str),
}
