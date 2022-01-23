use super::key;
use engine::utility::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Account {
	root: PathBuf,
	pub meta: Meta,
	pub key: Key,
	certificate: key::Certificate,
	private_key: key::PrivateKey,
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

	pub fn load(path: &Path) -> Result<Self> {
		let raw = std::fs::read_to_string(path)?;
		Ok(Meta::from_json(&raw)?)
	}

	pub fn save(&self, path: &Path) -> Result<()> {
		std::fs::write(path, self.to_json()?)?;
		Ok(())
	}

	fn from_json(json: &str) -> Result<Self> {
		let value: Self = serde_json::from_str(json)?;
		Ok(value)
	}

	fn to_json(&self) -> Result<String> {
		let mut json = serde_json::to_string_pretty(self)?;
		json = json.replace("  ", "\t");
		Ok(json)
	}
}

#[derive(Debug, Clone)]
pub enum KeyError {
	InvalidKeyType(String),
}
impl std::error::Error for KeyError {}
impl std::fmt::Display for KeyError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidKeyType(key) => write!(f, "Invalid key type: {}", key),
		}
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

	pub fn as_string(&self) -> Result<String> {
		use rsa::pkcs8::{ToPrivateKey, ToPublicKey};
		Ok(match self {
			Self::Private(private) => private.to_pkcs8_pem()?.to_string(),
			Self::Public(public) => public.to_public_key_pem()?,
		})
	}

	pub fn from_string(s: &str) -> Result<Self> {
		use rsa::pkcs8::{FromPrivateKey, FromPublicKey};
		if s.contains("PRIVATE") {
			Ok(Self::Private(rsa::RsaPrivateKey::from_pkcs8_pem(s)?))
		} else {
			Ok(Self::Public(rsa::RsaPublicKey::from_public_key_pem(s)?))
		}
	}

	pub fn load(path: &Path) -> Result<Self> {
		let key_string = std::fs::read_to_string(path)?;
		let key = Key::from_string(&key_string)?;
		Ok(key)
	}

	pub fn save(&self, path: &Path) -> Result<()> {
		std::fs::write(path, self.as_string()?)?;
		Ok(())
	}

	pub fn encrypt(&self, bytes: &[u8]) -> Result<Vec<u8>> {
		use rand::rngs::OsRng;
		use rsa::PublicKey;
		match self.public() {
			Self::Public(rsa) => {
				let mut rng = OsRng;
				let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
				Ok(rsa.encrypt(&mut rng, padding, &bytes)?)
			}
			private => Err(KeyError::InvalidKeyType(private.kind_str().to_owned()))?,
		}
	}

	pub fn decrypt(&self, bytes: &[u8]) -> Result<Result<Vec<u8>, rsa::errors::Error>, KeyError> {
		match self {
			Self::Private(rsa) => {
				let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
				Ok(rsa.decrypt(padding, &bytes))
			}
			public => Err(KeyError::InvalidKeyType(public.kind_str().to_owned())),
		}
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
	pub fn new(parent_dir: &Path, name: &String) -> Result<Self> {
		let id = uuid::Uuid::new_v4();
		let mut root = parent_dir.to_owned();
		root.push(id.to_string());
		let (certificate, private_key) = key::new()?;
		Ok(Self {
			root,
			meta: Meta {
				id,
				display_name: name.clone(),
			},
			key: Key::new(),
			certificate,
			private_key,
		})
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

	pub fn serialized_keys(&self) -> Result<(rustls::Certificate, rustls::PrivateKey)> {
		Ok((
			self.certificate.serialized()?,
			self.private_key.serialized()?,
		))
	}

	pub fn save(&self) -> Result<()> {
		use crate::common::utility::DataFile;
		std::fs::create_dir_all(&self.root)?;
		self.meta.save(&Meta::make_path(&self.root))?;
		self.key.save(&Key::make_path(&self.root))?;
		self.certificate.save(&self.root)?;
		self.private_key.save(&self.root)?;
		Ok(())
	}

	pub fn load(path: &Path) -> Result<Self> {
		use crate::common::utility::DataFile;
		let meta = Meta::load(&Meta::make_path(path))?;
		let key = Key::load(&Key::make_path(path))?;
		let certificate = key::Certificate::load(&path)?;
		let private_key = key::PrivateKey::load(&path)?;
		Ok(Account {
			root: path.to_owned(),
			meta,
			key,
			certificate,
			private_key,
		})
	}
}
