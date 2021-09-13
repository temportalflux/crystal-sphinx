use crate::account;
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, RwLock},
};

/// A user whose authentication is pending.
pub struct User {
	address: SocketAddr,
	id: account::Id,
	public_key: account::Key,
	token: String,
}

pub type ArcLockAuthCache = Arc<RwLock<AuthCache>>;

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
}
