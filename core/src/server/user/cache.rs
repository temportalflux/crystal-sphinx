use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, RwLock},
};

pub trait NetAddressable {
	fn address(&self) -> &SocketAddr;
}

pub struct Cache<TValue> {
	users: HashMap<SocketAddr, TValue>,
}

impl<TValue> Default for Cache<TValue> {
	fn default() -> Self {
		Self {
			users: HashMap::new(),
		}
	}
}

impl<TValue> Cache<TValue> {
	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl<TValue> Cache<TValue>
where
	TValue: NetAddressable,
{
	pub fn insert(&mut self, value: TValue) {
		self.users.insert(value.address().clone(), value);
	}

	pub fn remove(&mut self, address: &SocketAddr) -> Option<TValue> {
		self.users.remove(address)
	}
}
