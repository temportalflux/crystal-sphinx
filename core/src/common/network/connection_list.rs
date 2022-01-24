use crate::account::key::Certificate;
use engine::socknet::{
	connection::{self, Connection},
	utility::JoinHandleList,
};
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, RwLock},
};

pub struct ConnectionList {
	connections: HashMap<SocketAddr, Arc<Connection>>,
	#[allow(dead_code)]
	handles: Arc<JoinHandleList>,
}

impl ConnectionList {
	pub fn new(connection_receiver: connection::Receiver) -> Arc<RwLock<Self>> {
		let handles = Arc::new(JoinHandleList::new());
		let list = Arc::new(RwLock::new(Self {
			connections: HashMap::new(),
			handles: handles.clone(),
		}));

		let async_list = list.clone();
		handles.spawn(async move {
			while let Ok(connection) = connection_receiver.recv().await {
				let identity = match connection.peer_identity() {
					Some(identity) => identity,
					None => continue,
				};
				let certs = match identity.downcast::<Vec<rustls::Certificate>>() {
					Ok(certs) => certs,
					Err(_) => continue,
				};
				log::info!(target: "network", "connected to address={} identity={}", connection.remote_address(), Certificate::fingerprint(&certs[0]));
				async_list.write().unwrap().insert(connection);
			}
			Ok(())
		});

		list
	}

	pub fn insert(&mut self, connection: Arc<Connection>) {
		self.connections
			.insert(connection.remote_address(), connection);
	}
}
