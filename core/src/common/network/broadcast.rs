use crate::common::network::ConnectionList;
use engine::socknet::{connection::Connection, stream, utility::PinFutureResult};
use std::{
	collections::HashSet,
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

pub struct Broadcast<T> {
	connection_list: Arc<RwLock<ConnectionList>>,
	ignored_addresses: HashSet<SocketAddr>,
	on_established: Option<Arc<dyn Fn(T) -> PinFutureResult<()> + 'static + Send + Sync>>,
	_marker: std::marker::PhantomData<T>,
}

impl<T> Broadcast<T>
where
	T: stream::handler::Initiator
		+ From<stream::send::Context<T::Builder>>
		+ 'static
		+ Sized
		+ Send,
	T::Builder: stream::send::Builder + Send + Sync + 'static,
{
	pub fn new(connection_list: Arc<RwLock<ConnectionList>>) -> Self {
		Self {
			connection_list,
			ignored_addresses: HashSet::new(),
			on_established: None,
			_marker: std::marker::PhantomData::<T>::default(),
		}
	}

	pub fn ignore(mut self, connection: Arc<Connection>) -> Self {
		self.ignored_addresses.insert(connection.remote_address());
		self
	}

	pub fn with_on_established<F>(mut self, when_established: F) -> Self
	where
		F: Fn(T) -> PinFutureResult<()> + 'static + Send + Sync,
	{
		let when_established = Arc::new(when_established);
		self.on_established = Some(when_established);
		self
	}

	pub fn make_target_list(&self) -> Vec<Weak<Connection>> {
		let connection_list = self.connection_list.read().unwrap();
		connection_list
			.all()
			.iter()
			.filter_map(
				|(address, connection)| match self.ignored_addresses.contains(&address) {
					true => None,
					false => Some(connection.clone()),
				},
			)
			.collect()
	}

	pub fn open(self) {
		assert!(self.on_established.is_some());
		use stream::Identifier;
		let log = format!("broadcast({})", T::Builder::unique_id());
		for connection in self.make_target_list() {
			let when_established = self.on_established.as_ref().unwrap().clone();
			engine::task::spawn(log.clone(), async move {
				let handler = T::open(&connection)?.await?;
				(when_established)(handler).await?;
				Ok(())
			});
		}
	}
}
