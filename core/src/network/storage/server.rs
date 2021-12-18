use crate::{account, world::ArcLockDatabase};
use engine::utility::{AnyError, VoidResult};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

static LOG: &'static str = "server";

pub mod user;

pub type ArcLockServer = Arc<RwLock<Server>>;
pub struct Server {
	root_dir: PathBuf,
	auth_key: account::Key,
	saved_users: HashMap<account::Id, Arc<RwLock<user::saved::User>>>,
	world: Option<ArcLockDatabase>,
}

impl Server {
	pub fn load(save_name: &str) -> Result<Self, AnyError> {
		let mut savegame_path = std::env::current_dir().unwrap();
		savegame_path.push("saves");
		savegame_path.push(save_name);

		if !savegame_path.exists() {
			Self::create(&savegame_path)?;
		}
		log::info!(target: LOG, "Loading data");
		Ok(Self {
			root_dir: savegame_path.to_owned(),
			auth_key: account::Key::load(&Self::auth_key_path(savegame_path.to_owned()))?,
			saved_users: Self::load_saved_users(&Self::players_dir_path(savegame_path.to_owned()))?,
			world: None,
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

	pub fn get_players_dir_path(&self) -> PathBuf {
		Self::players_dir_path(self.root_dir.clone())
	}

	fn world_name(&self) -> &str {
		self.root_dir.file_name().unwrap().to_str().unwrap()
	}

	fn load_saved_users(
		path: &Path,
	) -> Result<HashMap<account::Id, Arc<RwLock<user::saved::User>>>, AnyError> {
		std::fs::create_dir_all(path)?;
		let mut users = HashMap::new();
		for entry in std::fs::read_dir(path)? {
			let user_path = entry?.path();
			if user_path.is_dir() {
				match user::saved::User::load(&user_path) {
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

	pub fn add_user(&mut self, user: user::saved::User) {
		let id = user.id().clone();
		let arc_user = Arc::new(RwLock::new(user));
		let thread_user = arc_user.clone();
		std::thread::spawn(move || {
			if let Ok(user) = thread_user.read() {
				let _ = user.save();
			}
		});
		self.saved_users.insert(id, arc_user);
	}

	pub fn find_user(&self, id: &account::Id) -> Option<&Arc<RwLock<user::saved::User>>> {
		self.saved_users.get(id)
	}

	fn world_path(mut savegame_path: PathBuf) -> PathBuf {
		savegame_path.push("world");
		savegame_path
	}

	pub fn start_loading_world(&mut self) {
		use crate::world::Database;

		log::warn!(target: "world-loader", "Loading world \"{}\"", self.world_name());
		let world = Database::new(Self::world_path(self.root_dir.to_owned()));

		let arc_world = Arc::new(RwLock::new(world));
		let origin_res = Database::load_origin_chunk(&arc_world);
		assert!(origin_res.is_ok());

		self.world = Some(arc_world);
	}
}
