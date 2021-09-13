use crate::{
	account,
	engine::utility::{singleton, AnyError, VoidResult},
};
use std::{
	path::{Path, PathBuf},
	sync::{LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

static LOG: &'static str = "server";

pub struct Server {
	auth_key: account::Key,
}

impl Server {
	fn get() -> &'static RwLock<Option<Self>> {
		static mut INSTANCE: singleton::RwOptional<Server> = singleton::RwOptional::uninit();
		unsafe { INSTANCE.get() }
	}

	pub fn write() -> LockResult<RwLockWriteGuard<'static, Option<Self>>> {
		Self::get().write()
	}

	pub fn read() -> LockResult<RwLockReadGuard<'static, Option<Self>>> {
		Self::get().read()
	}
}

impl Server {
	pub fn load(savegame_path: &Path) -> Result<Self, AnyError> {
		if !savegame_path.exists() {
			Self::create(savegame_path)?;
		}
		log::info!(target: LOG, "Loading data");
		Ok(Self {
			auth_key: account::Key::load(&Self::auth_key_path(savegame_path.to_owned()))?,
		})
	}

	fn create(savegame_path: &Path) -> VoidResult {
		log::info!(target: LOG, "Creating data");
		std::fs::create_dir_all(savegame_path)?;
		account::Key::new().save(&Self::auth_key_path(savegame_path.to_owned()))?;
		Ok(())
	}

	fn auth_key_path(mut savegame_path: PathBuf) -> PathBuf {
		savegame_path.push("private_key.txt");
		savegame_path
	}

	pub fn auth_key(&self) -> &account::Key {
		&self.auth_key
	}
	
}
