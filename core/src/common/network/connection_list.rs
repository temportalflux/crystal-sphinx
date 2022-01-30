use engine::socknet::{
	connection::{self, Active, Connection},
	utility::JoinHandleList,
};
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

pub struct ConnectionList {
	connections: HashMap<SocketAddr, Weak<Connection>>,
	#[allow(dead_code)]
	handles: Arc<JoinHandleList>,
}

impl ConnectionList {
	pub fn new(connection_receiver: connection::event::Receiver) -> Arc<RwLock<Self>> {
		let handles = Arc::new(JoinHandleList::new());
		let list = Arc::new(RwLock::new(Self {
			connections: HashMap::new(),
			handles: handles.clone(),
		}));

		let async_list = list.clone();
		let target = "connection-list".to_owned();
		handles.spawn(target.clone(), async move {
			use connection::event::Event::*;
			while let Ok(event) = connection_receiver.recv().await {
				match event {
					Created(connection) => {
						let arc = Connection::upgrade(&connection)?;
						log::info!(
							target: &target,
							"connected to address({}) identity({})",
							arc.remote_address(),
							arc.fingerprint()?
						);

						let mut list = async_list.write().unwrap();
						list.insert(arc.remote_address(), connection);
					}
					Dropped(address) => {
						log::info!(target: &target, "disconnected from address({})", address);

						let mut list = async_list.write().unwrap();
						list.remove(&address);
					}
				}
			}
			Ok(())
		});

		list
	}

	pub fn insert(&mut self, address: SocketAddr, connection: Weak<Connection>) {
		self.connections.insert(address, connection);
	}

	pub fn remove(&mut self, address: &SocketAddr) {
		self.connections.remove(&address);
	}

	pub fn all(&self) -> &HashMap<SocketAddr, Weak<Connection>> {
		&self.connections
	}
}
