use engine::utility::{AnyError, VoidResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Account {
	root: PathBuf,
	pub meta: Meta,
	pub key: Key,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
	pub id: super::Id,
	pub display_name: String,
}

impl std::fmt::Display for Meta {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "AccountId(name={}, id={})", self.display_name, self.id)
	}
}

impl Meta {
	pub fn make_path(parent_dir: &Path) -> PathBuf {
		let mut path = parent_dir.to_owned();
		path.push("meta.json");
		path
	}

	pub fn load(path: &Path) -> Result<Self, AnyError> {
		let raw = std::fs::read_to_string(path)?;
		Ok(Meta::from_json(&raw)?)
	}

	fn from_json(json: &str) -> Result<Self, AnyError> {
		let value: Self = serde_json::from_str(json)?;
		Ok(value)
	}

	fn to_json(&self) -> Result<String, AnyError> {
		let mut json = serde_json::to_string_pretty(self)?;
		json = json.replace("  ", "\t");
		Ok(json)
	}
}

#[derive(Debug, Clone)]
pub enum Key {
	Private(rsa::RsaPrivateKey),
	Public(rsa::RsaPublicKey),
}

impl Key {
	pub fn new() -> Self {
		use rand::rngs::OsRng;
		let mut rng = OsRng;
		let bits = 2048;
		Self::Private(rsa::RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key"))
	}

	pub fn make_path(parent_dir: &Path) -> PathBuf {
		let mut path = parent_dir.to_owned();
		path.push("key.txt");
		path
	}

	pub fn kind_str(&self) -> &'static str {
		match self {
			Self::Private(_) => "Private",
			Self::Public(_) => "Public",
		}
	}

	pub fn public(&self) -> Self {
		match self {
			Self::Private(private) => Self::Public(rsa::RsaPublicKey::from(private)),
			Self::Public(_) => self.clone(),
		}
	}

	pub fn as_string(&self) -> Result<String, AnyError> {
		use rsa::pkcs8::{ToPrivateKey, ToPublicKey};
		Ok(match self {
			Self::Private(private) => private.to_pkcs8_pem()?.to_string(),
			Self::Public(public) => public.to_public_key_pem()?,
		})
	}

	pub fn from_string(s: &str) -> Result<Self, AnyError> {
		use rsa::pkcs8::{FromPrivateKey, FromPublicKey};
		if s.contains("PRIVATE") {
			Ok(Self::Private(rsa::RsaPrivateKey::from_pkcs8_pem(s)?))
		} else {
			Ok(Self::Public(rsa::RsaPublicKey::from_public_key_pem(s)?))
		}
	}

	pub fn load(path: &Path) -> Result<Self, AnyError> {
		let key_string = std::fs::read_to_string(path)?;
		let key = Key::from_string(&key_string)?;
		Ok(key)
	}

	pub fn save(&self, path: &Path) -> VoidResult {
		std::fs::write(path, self.as_string()?)?;
		Ok(())
	}
}

impl std::fmt::Display for Account {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"Account(id:{}, display_name:{}, key:{})",
			self.meta.id,
			self.meta.display_name,
			self.key.kind_str()
		)
	}
}

impl Account {
	/// Creates a new account, complete with a generated RSA key.
	pub fn new(parent_dir: &Path, name: &String) -> Self {
		let id = uuid::Uuid::new_v4();
		let mut root = parent_dir.to_owned();
		root.push(id.to_string());
		Self {
			root,
			meta: Meta {
				id,
				display_name: name.clone(),
			},
			key: Key::new(),
		}
	}

	pub fn id(&self) -> &super::Id {
		&self.meta.id
	}

	pub fn public_key(&self) -> Key {
		self.key.public()
	}

	pub fn display_name(&self) -> &String {
		&self.meta.display_name
	}

	pub fn save(&self) -> VoidResult {
		std::fs::create_dir_all(&self.root)?;
		std::fs::write(&Meta::make_path(&self.root), self.meta.to_json()?)?;
		self.key.save(&Key::make_path(&self.root))?;
		Ok(())
	}

	pub fn load(path: &Path) -> Result<Self, AnyError> {
		let meta = Meta::load(&Meta::make_path(path))?;
		let key = Key::load(&Key::make_path(path))?;
		Ok(Account {
			root: path.to_owned(),
			meta,
			key,
		})
	}
}
