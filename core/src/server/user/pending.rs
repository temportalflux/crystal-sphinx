use crate::account;
use std::{
	collections::HashMap,
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

pub type ArcLockAuthCache = Arc<RwLock<AuthCache>>;

/// A user whose authentication is pending.
pub struct User {
	address: SocketAddr,
	id: account::Id,
	token: String,

	timeout_thread: Option<thread::JoinHandle<()>>,
	timeout_exit: Arc<AtomicBool>,
}

impl User {
	pub fn new(address: SocketAddr, id: account::Id, token: String) -> Self {
		Self {
			address,
			id,
			token,
			timeout_thread: None,
			timeout_exit: Arc::new(AtomicBool::new(false)),
		}
	}

	pub fn address(&self) -> &SocketAddr {
		&self.address
	}

	pub fn id(&self) -> &account::Id {
		&self.id
	}

	pub fn token(&self) -> &String {
		&self.token
	}

	pub fn start_timeout(&mut self, cache: &ArcLockAuthCache) {
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
								target: crate::server::LOG,
								"Authentication for {} has timed out",
								address
							);
							if let Ok(mut auth_cache) = auth_cache.write() {
								let _ = auth_cache.remove_pending_user(&address);
								let _ = engine::network::Network::kick(&address);
							}
							return;
						}
						thread::sleep(USER_TIMEOUT_CYCLE_MS);
					}
					log::trace!(
						target: crate::server::LOG,
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

impl Drop for User {
	fn drop(&mut self) {
		self.stop_timeout();
	}
}

/// Caches pending users and their auth tokens until the users are authenticated or disconnected.
pub struct AuthCache {
	pending_users: HashMap<SocketAddr, User>,
}

impl Default for AuthCache {
	fn default() -> Self {
		Self {
			pending_users: HashMap::new(),
		}
	}
}

impl AuthCache {
	pub fn arclocked(self) -> ArcLockAuthCache {
		Arc::new(RwLock::new(self))
	}

	pub fn add_pending_user(&mut self, mut pending: User, cache: &ArcLockAuthCache) {
		pending.start_timeout(cache);
		self.pending_users.insert(pending.address.clone(), pending);
	}

	pub fn remove_pending_user(&mut self, address: &SocketAddr) -> Option<User> {
		self.pending_users.remove(address)
	}
}
