use bus::{Bus, BusReader};
use socknet::{
	connection::{self, event, Active, Connection},
	utility::JoinHandleList,
};
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, Mutex, RwLock, Weak},
};

pub struct List {
	connections: HashMap<SocketAddr, Weak<Connection>>,
	#[allow(dead_code)]
	handles: Arc<JoinHandleList>,
	event_dispatcher: Arc<Mutex<Bus<super::Event>>>,
}

impl List {
	pub fn new(receiver: event::Receiver) -> Arc<RwLock<Self>> {
		let handles = Arc::new(JoinHandleList::new());
		let list = Arc::new(RwLock::new(Self {
			connections: HashMap::new(),
			handles: handles.clone(),
			event_dispatcher: Arc::new(Mutex::new(Bus::new(100))),
		}));

		let async_list = list.clone();
		let target = "connection-list".to_owned();
		handles.spawn(target.clone(), async move {
			use connection::event::Event::*;
			while let Ok(event) = receiver.recv().await {
				match event {
					Created(connection) => {
						let arc = Connection::upgrade(&connection)?;
						log::info!(
							target: &target,
							"connected to address({}) identity({})",
							arc.remote_address(),
							arc.fingerprint()?
						);
						let is_local = arc.is_local();

						let mut list = async_list.write().unwrap();
						list.insert(arc.remote_address(), connection.clone());

						list.broadcast(super::Event::Created(
							arc.remote_address(),
							connection,
							is_local,
						));
					}
					Dropped(address) => {
						log::info!(target: &target, "disconnected from address({})", address);

						let mut list = async_list.write().unwrap();
						list.remove(&address);

						list.broadcast(super::Event::Dropped(address));
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

	pub fn first(&self) -> Option<&Weak<Connection>> {
		self.connections.iter().next().map(|(_, conn)| conn)
	}

	pub fn add_recv(&mut self) -> BusReader<super::Event> {
		self.event_dispatcher.lock().unwrap().add_rx()
	}

	/// Non-blocking async-spawning broadcast to reliably send some event through the bus.
	pub fn broadcast(&self, event: super::Event) {
		let arclock_dispatcher = self.event_dispatcher.clone();
		engine::task::spawn("connection-list".to_owned(), async move {
			let mut dispatcher = arclock_dispatcher.lock().unwrap();
			// This is a blocking call that will wait until there is room in the bus to send the event.
			dispatcher.broadcast(event);
			Ok(())
		});
	}
}
