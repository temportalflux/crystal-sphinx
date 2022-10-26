use crate::{
	common::account::{self, key},
	entity::{self, ArcLockEntityWorld},
	server::user,
	server::world::{chunk, Database},
};
use anyhow::{Context, Result};
use engine::{Engine, EngineSystem};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

static LOG: &'static str = "server";

pub struct Storage {
	root_dir: PathBuf,

	certificate: key::Certificate,
	private_key: key::PrivateKey,
	users: HashMap<account::Id, Arc<RwLock<user::Active>>>,

	database: Option<Arc<RwLock<Database>>>,
	systems: Vec<Arc<RwLock<dyn EngineSystem + Send + Sync>>>,
}

impl Storage {
	#[profiling::function]
	pub fn load(save_name: &str) -> Result<Self> {
		use crate::common::utility::DataFile;
		let mut savegame_path = std::env::current_dir().unwrap();
		savegame_path.push("saves");
		savegame_path.push(save_name);

		if !savegame_path.exists() {
			Self::create(&savegame_path).context("generating server data")?;
		}
		log::info!(target: LOG, "Loading data");
		let certificate =
			key::Certificate::load(&savegame_path).context("loading server certificate")?;
		let private_key =
			key::PrivateKey::load(&savegame_path).context("loading server private key")?;
		Ok(Self {
			root_dir: savegame_path.to_owned(),

			certificate,
			private_key,
			users: Self::load_users(&Self::players_dir_path(savegame_path.to_owned()))
				.context("loading users")?,

			database: None,
			systems: vec![],
		})
	}

	fn create(root: &Path) -> Result<()> {
		use crate::common::utility::DataFile;
		log::info!(target: LOG, "Creating data");
		std::fs::create_dir_all(root)?;

		let (_, certificate, private_key) = key::create_pem()?;
		std::fs::write(&key::Certificate::make_path(&root), certificate)?;
		std::fs::write(&key::PrivateKey::make_path(&root), private_key)?;

		Ok(())
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

	fn load_users(path: &Path) -> Result<HashMap<account::Id, Arc<RwLock<user::Active>>>> {
		std::fs::create_dir_all(path)?;
		let mut users = HashMap::new();
		for entry in std::fs::read_dir(path)? {
			let user_path = entry?.path();
			if user_path.is_dir() {
				match user::Active::load(&user_path) {
					Ok(user) => {
						log::info!(target: LOG, "Loaded user {}", user.account().id());
						users.insert(user.account().id().clone(), Arc::new(RwLock::new(user)));
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

	pub fn add_user(&mut self, id: account::Id, user: Arc<RwLock<user::Active>>) {
		self.users.insert(id, user.clone());
		engine::task::spawn(LOG.to_string(), async move {
			user.read().unwrap().save()?;
			Ok(())
		});
	}

	pub fn find_user(&self, id: &account::Id) -> Option<&Arc<RwLock<user::Active>>> {
		self.users.get(id)
	}

	fn world_path(mut savegame_path: PathBuf) -> PathBuf {
		savegame_path.push("world");
		savegame_path
	}

	pub fn initialize_systems(&mut self, entity_world: &ArcLockEntityWorld) {
		self.add_system(entity::system::UserChunkTicketUpdater::new(&entity_world));
	}

	pub fn add_system<T>(&mut self, system: T)
	where
		T: EngineSystem + 'static + Send + Sync,
	{
		let system = Arc::new(RwLock::new(system));
		{
			let mut engine = Engine::get().write().unwrap();
			engine.add_weak_system(Arc::downgrade(&system));
		}
		self.systems.push(system);
	}

	pub fn get_keys(&self) -> Result<(rustls::Certificate, rustls::PrivateKey)> {
		let certificate: rustls::Certificate = self.certificate.clone().into();
		let private_key: rustls::PrivateKey = self.private_key.clone().into();
		Ok((certificate, private_key))
	}

	#[profiling::function]
	pub fn start_loading_world(&mut self) -> anyhow::Result<()> {
		log::warn!(target: "world-loader", "Loading world \"{}\"", self.world_name());
		let database = Database::new(Self::world_path(self.root_dir.to_owned()))?;

		let arc_database = Arc::new(RwLock::new(database));
		let origin_res = Database::load_origin_chunk(&arc_database);
		assert!(origin_res.is_ok());

		self.database = Some(arc_database);
		Ok(())
	}

	pub fn chunk_cache(&self) -> chunk::cache::ArcLock {
		let database = self.database.as_ref().unwrap().read().unwrap();
		database.chunk_cache().clone()
	}
}
