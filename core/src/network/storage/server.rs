use crate::{
	account,
	common::account::key,
	entity::{self, ArcLockEntityWorld},
	server::user::User,
	server::world::Database,
};
use engine::{
	network::endpoint,
	utility::{Context, Result},
	Engine, EngineSystem,
};
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

	// OLD
	auth_key: account::Key,
	saved_users: HashMap<account::Id, Arc<RwLock<user::saved::User>>>,

	// NEW
	certificate: key::Certificate,
	private_key: key::PrivateKey,
	users: HashMap<account::Id, Arc<RwLock<User>>>,

	database: Option<Arc<RwLock<Database>>>,
	systems: Vec<Arc<RwLock<dyn EngineSystem + Send + Sync>>>,
}

impl Server {
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

			// OLD
			auth_key: account::Key::load(&Self::auth_key_path(savegame_path.to_owned()))
				.context("loading old server auth key")?,
			saved_users: HashMap::new(),
			// NEW
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

		account::Key::new().save(&Self::auth_key_path(root.to_owned()))?;

		let (_, certificate, private_key) = key::create_pem()?;
		std::fs::write(&key::Certificate::make_path(&root), certificate)?;
		std::fs::write(&key::PrivateKey::make_path(&root), private_key)?;

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

	fn load_users(path: &Path) -> Result<HashMap<account::Id, Arc<RwLock<User>>>> {
		std::fs::create_dir_all(path)?;
		let mut users = HashMap::new();
		for entry in std::fs::read_dir(path)? {
			let user_path = entry?.path();
			if user_path.is_dir() {
				match User::load(&user_path) {
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

	pub fn add_saved_user(&mut self, user: user::saved::User) {
		let id = user.id().clone();
		let arc_user = Arc::new(RwLock::new(user));
		let thread_user = arc_user.clone();
		std::thread::spawn(move || {
			profiling::register_thread!("save-user");
			if let Ok(user) = thread_user.read() {
				let _ = user.save();
			}
		});
		self.saved_users.insert(id, arc_user);
	}

	pub fn find_saved_user(&self, id: &account::Id) -> Option<&Arc<RwLock<user::saved::User>>> {
		self.saved_users.get(id)
	}

	pub fn add_user(&mut self, id: account::Id, user: Arc<RwLock<User>>) {
		self.users.insert(id, user.clone());
		engine::task::spawn(LOG.to_string(), async move {
			user.read().unwrap().save()?;
			Ok(())
		});
	}

	pub fn find_user(&self, id: &account::Id) -> Option<&Arc<RwLock<User>>> {
		self.users.get(id)
	}

	fn world_path(mut savegame_path: PathBuf) -> PathBuf {
		savegame_path.push("world");
		savegame_path
	}

	pub fn initialize_systems(&mut self, entity_world: &ArcLockEntityWorld) {
		let chunk_cache = {
			let database = self.database.as_ref().unwrap().read().unwrap();
			Arc::downgrade(database.chunk_cache())
		};
		self.add_system(entity::system::Replicator::new(chunk_cache, &entity_world));
		self.add_system(entity::system::UserChunkTicketUpdater::new(&entity_world));
	}

	fn add_system<T>(&mut self, system: T)
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
	pub fn start_loading_world(&mut self) {
		log::warn!(target: "world-loader", "Loading world \"{}\"", self.world_name());
		let database = Database::new(Self::world_path(self.root_dir.to_owned()));

		let arc_database = Arc::new(RwLock::new(database));
		let origin_res = Database::load_origin_chunk(&arc_database);
		assert!(origin_res.is_ok());

		self.database = Some(arc_database);
	}
}

// Implementation of `ClientCertVerifier` that verifies everything as trustworthy.
pub struct AllowAnyClient;

impl AllowAnyClient {
	pub fn new() -> Arc<Self> {
		Arc::new(Self)
	}
}

impl rustls::server::ClientCertVerifier for AllowAnyClient {
	fn client_auth_root_subjects(&self) -> Option<rustls::DistinguishedNames> {
		Some(vec![])
	}

	fn verify_client_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		_now: std::time::SystemTime,
	) -> Result<rustls::server::ClientCertVerified, rustls::Error> {
		log::info!(target: "server", "Ignoring verification of client certificate");
		Ok(rustls::server::ClientCertVerified::assertion())
	}
}
