use crate::account;
use engine::utility::{singleton, AnyError, VoidResult};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

static LOG: &'static str = "server";

pub mod user;

pub struct Server {
	auth_key: account::Key,
	saved_users: HashMap<account::Id, Arc<RwLock<user::saved::User>>>,
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
			saved_users: Self::load_saved_users(&Self::players_dir_path(savegame_path.to_owned()))?,
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

	fn players_dir_path(mut savegame_path: PathBuf) -> PathBuf {
		savegame_path.push("players");
		savegame_path
	}

	fn load_saved_users(
		path: &Path,
	) -> Result<HashMap<account::Id, Arc<RwLock<user::saved::User>>>, AnyError> {
		std::fs::create_dir_all(path)?;
		let mut users = HashMap::new();
		for entry in std::fs::read_dir(path)? {
			let user_path = entry?.path();
			if user_path.is_dir() {
				match user::saved::User::from(&user_path) {
					Ok(user) => {
						log::info!(target: LOG, "Loaded user {}", user.id());
						users.insert(user.id().clone(), Arc::new(RwLock::new(user)));
					}
					Err(err) => {
						log::warn!(
							target: LOG,
							"Failed to load user {}: {}",
							user_path.display(),
							err
						);
					}
				}
			}
		}
		Ok(users)
	}

	pub fn find_user(&self, id: &account::Id) -> Option<&Arc<RwLock<user::saved::User>>> {
		self.saved_users.get(id)
	}
}
