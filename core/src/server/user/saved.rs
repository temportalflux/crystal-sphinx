use crate::account;
use engine::utility::{AnyError, VoidResult};
use std::path::{Path, PathBuf};

pub struct User {
	save_location: PathBuf,
	meta: account::Meta,
	key: account::Key,
}

impl User {
	pub fn new(pending: &super::pending::User, mut save_location: PathBuf) -> Self {
		save_location.push(pending.meta().id.to_string());
		Self {
			save_location,
			meta: pending.meta().clone(),
			key: pending.public_key().clone(),
		}
	}

	pub fn load(dir: &Path) -> Result<Self, AnyError> {
		let meta = account::Meta::load(&account::Meta::make_path(dir))?;
		let key = account::Key::load(&account::Key::make_path(dir))?;
		Ok(Self {
			save_location: dir.to_owned(),
			meta,
			key,
		})
	}

	pub fn save(&self) -> VoidResult {
		use engine::Application;
		log::debug!(
			target: crate::CrystalSphinx::name(),
			"Saving user {} to disk",
			self.meta
		);
		std::fs::create_dir_all(&self.save_location)?;
		self.meta
			.save(&account::Meta::make_path(&self.save_location))?;
		self.key
			.save(&account::Key::make_path(&self.save_location))?;
		Ok(())
	}

	pub fn id(&self) -> &account::Id {
		&self.meta.id
	}

	pub fn public_key(&self) -> account::Key {
		self.key.public()
	}
}