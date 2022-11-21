use crate::common::utility::ThreadHandle;
use crate::common::world::Database;
use crate::server::world::{
	chunk::{thread, ticket, Level, Ticket},
	Settings,
};
use engine::math::nalgebra::Point3;
use std::{
	path::PathBuf,
	sync::{Arc, RwLock, Weak},
};

/// [SERVER ONLY] Loads chunks as tickets are processed/updated, either from disk or via a generator.
pub struct Loader {
	#[allow(dead_code)]
	settings: Settings,
	/// The channel to send tickets through to trigger their relevant chunks to be loaded.
	send_tickets: ticket::Sender,
	/// The thread which handles the loading of chunks.
	/// When the loader is dropped, this handle will force the thread to stop.
	_chunk_thread_handle: ThreadHandle,
	/// Tickets which keep chunks loaded for the lifetime of the world.
	/// This should not be for storing tickets for other owners,
	/// but rather for chunks like the origin/spawn area which are loaded all the time.
	world_tickets: Vec<Arc<Ticket>>,
}

impl Loader {
	pub fn new(root_path: PathBuf, database: Weak<RwLock<Database>>) -> anyhow::Result<Self> {
		let settings = Settings::load(&root_path).unwrap();

		let (send_tickets, recv_tickets) = engine::channels::mpsc::unbounded();

		let thread_handle = thread::start(root_path, recv_tickets, database)?;

		let mut loader = Self {
			settings,
			send_tickets,
			_chunk_thread_handle: thread_handle,
			world_tickets: vec![],
		};

		loader.world_tickets.push(loader.submit(Ticket {
			coordinate: Point3::new(0, 0, 0),
			level: (Level::Ticking, 2).into(),
		}));

		Ok(loader)
	}

	pub fn submit(&self, ticket: Ticket) -> Arc<Ticket> {
		let arc = Arc::new(ticket);
		self.send_tickets.try_send(Arc::downgrade(&arc)).unwrap();
		arc
	}
}
