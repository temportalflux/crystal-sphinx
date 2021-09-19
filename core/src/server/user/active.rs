use crate::account;
use std::{
	net::SocketAddr,
	sync::{Arc, RwLock},
};

pub type Cache = super::Cache<User>;
pub type ArcLockCache = Arc<RwLock<Cache>>;

pub struct User {
	address: SocketAddr,
	id: account::Id,
}

impl From<super::pending::User> for User {
	fn from(pending: super::pending::User) -> Self {
		Self {
			address: pending.address().clone(),
			id: pending.id().clone(),
		}
	}
}

impl super::NetAddressable for User {
	fn address(&self) -> &SocketAddr {
		&self.address
	}
}
