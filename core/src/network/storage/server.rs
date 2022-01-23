use crate::{
	account::{self, key},
	entity::{self, ArcLockEntityWorld},
	server::world::Database,
};
use engine::{utility::Result, Engine, EngineSystem};
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
	certificate: key::Certificate,
	private_key: key::PrivateKey,
	saved_users: HashMap<account::Id, Arc<RwLock<user::saved::User>>>,

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
			Self::create(&savegame_path)?;
		}
		log::info!(target: LOG, "Loading data");
		let certificate = key::Certificate::load(&savegame_path)?;
		let private_key = key::PrivateKey::load(&savegame_path)?;
		Ok(Self {
			root_dir: savegame_path.to_owned(),
			auth_key: account::Key::load(&Self::auth_key_path(savegame_path.to_owned()))?,
			certificate,
			private_key,
			saved_users: Self::load_saved_users(&Self::players_dir_path(savegame_path.to_owned()))?,
			database: None,
			systems: vec![],
		})
	}

	fn create(savegame_path: &Path) -> Result<()> {
		use crate::common::utility::DataFile;
		log::info!(target: LOG, "Creating data");
		std::fs::create_dir_all(savegame_path)?;
		account::Key::new().save(&Self::auth_key_path(savegame_path.to_owned()))?;
		let (certificate, private_key) = key::new()?;
		certificate.save(&savegame_path)?;
		private_key.save(&savegame_path)?;
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
	) -> Result<HashMap<account::Id, Arc<RwLock<user::saved::User>>>> {
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
			profiling::register_thread!("save-user");
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
		Engine::get()
			.write()
			.unwrap()
			.add_weak_system(Arc::downgrade(&system));
		self.systems.push(system);
	}

	pub fn create_config(&self) -> Result<quinn::ServerConfig> {
		let cert = self.certificate.serialized()?;
		let key = self.private_key.serialized()?;
		log::debug!(target: "server", "local identity={}", key::Certificate::fingerprint(&cert));

		let core_config = rustls::ServerConfig::builder()
			.with_safe_defaults()
			.with_client_cert_verifier(AllowAnyClient::new())
			.with_single_cert(vec![cert], key)?;

		Ok(quinn::ServerConfig::with_crypto(Arc::new(core_config)))
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
struct AllowAnyClient;

impl AllowAnyClient {
	fn new() -> Arc<Self> {
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
