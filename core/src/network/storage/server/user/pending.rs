use crate::{account, network::storage::server};
use std::{
	net::SocketAddr,
	sync::{
		atomic::{self, AtomicBool},
		Arc, RwLock,
	},
	thread,
	time::{Duration, Instant},
};

// Users connected but not authenticated will timeout after 10 seconds of not being authenticated
static USER_TIMEOUT_TOTAL_MS: Duration = Duration::from_millis(10 * 1000);
// Check the timeout of a pending user every half second
static USER_TIMEOUT_CYCLE_MS: Duration = Duration::from_millis(500);

/// A user whose authentication is pending.
pub struct User {
	address: SocketAddr,
	meta: account::Meta,
	public_key: account::Key,
	token: String,

	timeout_thread: Option<thread::JoinHandle<()>>,
	timeout_exit: Arc<AtomicBool>,
}

impl User {
	pub fn new(
		address: SocketAddr,
		meta: account::Meta,
		public_key: account::Key,
		token: String,
	) -> Self {
		Self {
			address,
			meta,
			public_key,
			token,
			timeout_thread: None,
			timeout_exit: Arc::new(AtomicBool::new(false)),
		}
	}

	pub fn address(&self) -> &SocketAddr {
		&self.address
	}

	pub fn meta(&self) -> &account::Meta {
		&self.meta
	}

	pub fn id(&self) -> &account::Id {
		&self.meta.id
	}

	pub fn public_key(&self) -> &account::Key {
		&self.public_key
	}

	pub fn token(&self) -> &String {
		&self.token
	}

	pub fn start_timeout(&mut self, cache: &ArcLockCache) {
		let auth_cache = cache.clone();
		let address = self.address.clone();
		let exit_flag = self.timeout_exit.clone();
		let thread_name = format!("pending-user:{}", self.address);
		self.timeout_thread = Some(
			thread::Builder::new()
				.name(thread_name.clone())
				.spawn(move || {
					let start_time = Instant::now();
					while !exit_flag.load(atomic::Ordering::Relaxed) {
						if Instant::now().duration_since(start_time.clone())
							>= USER_TIMEOUT_TOTAL_MS
						{
							log::info!(
								target: server::LOG,
								"Authentication for {} has timed out",
								address
							);
							if let Ok(mut auth_cache) = auth_cache.write() {
								let _ = auth_cache.remove(&address);
								let _ = engine::network::Network::kick(&address);
							}
							return;
						}
						thread::sleep(USER_TIMEOUT_CYCLE_MS);
					}
					log::trace!(
						target: server::LOG,
						"thread \"{}\" has concluded",
						thread_name
					);
				})
				.unwrap(),
		);
	}

	pub fn stop_timeout(&self) {
		self.timeout_exit.store(true, atomic::Ordering::Relaxed);
	}
}

impl super::NetAddressable for User {
	fn address(&self) -> &SocketAddr {
		&self.address
	}
}

impl Drop for User {
	fn drop(&mut self) {
		self.stop_timeout();
	}
}

/// Caches pending users and their auth tokens until the users are authenticated or disconnected.
pub type Cache = super::Cache<User>;
pub type ArcLockCache = Arc<RwLock<Cache>>;
