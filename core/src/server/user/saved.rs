use crate::account;
use engine::utility::AnyError;
use std::path::Path;

pub struct User {
	meta: account::Meta,
	key: account::Key,
}

impl User {
	pub fn from(dir: &Path) -> Result<Self, AnyError> {
		let meta = account::Meta::load(&account::Meta::make_path(dir))?;
		let key = account::Key::load(&account::Key::make_path(dir))?;
		Ok(Self { meta, key })
	}

	pub fn id(&self) -> &account::Id {
		&self.meta.id
	}

	pub fn public_key(&self) -> account::Key {
		self.key.public()
	}
}
