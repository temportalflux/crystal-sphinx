use super::chunk;
use super::Settings;
use engine::math::nalgebra::Point3;
use engine::utility::AnyError;
use std::{
	path::PathBuf,
	sync::{Arc, Mutex, RwLock},
};

pub type ArcLockDatabase = Arc<RwLock<Database>>;

/// The data about a world (its chunks, settings, etc).
/// Exists on the server, does not contain presentational/graphical data.
pub struct Database {
	_settings: Settings,
	chunk_cache: chunk::ArcLockCache,
	load_request_sender: chunk::LoadRequestSender,
	// When this is dropped, the loading thread stops.
	_chunk_thread_handle: chunk::ThreadHandle,

	held_tickets: Vec<chunk::ArctexTicket>,
}

impl Database {
	pub fn new(root_path: PathBuf) -> Self {
		let settings = Settings::load(&root_path).unwrap();

		let chunk_cache = Arc::new(RwLock::new(chunk::Cache::new(&settings)));

		let (load_request_sender, load_request_receiver) = crossbeam_channel::unbounded();
		let thread_handle = chunk::Cache::start_loading_thread(load_request_receiver, &chunk_cache);

		Self {
			_settings: settings,
			chunk_cache,
			load_request_sender,
			_chunk_thread_handle: thread_handle,

			held_tickets: Vec::new(),
		}
	}

	pub fn chunk_cache(&self) -> &chunk::ArcLockCache {
		&self.chunk_cache
	}

	pub fn load_origin_chunk(arc_world: &ArcLockDatabase) {
		let ticket_result = arc_world
			.read()
			.unwrap()
			.create_chunk_ticket(chunk::Ticket::new(Point3::new(0, 0, 0)));
		match ticket_result {
			Ok(arc_ticket) => {
				arc_world.write().unwrap().held_tickets.push(arc_ticket);
			}
			Err(err) => {
				log::error!(target: "world", "Failed to load origin chunk: {}", err);
			}
		}
	}

	pub fn create_chunk_ticket(
		&self,
		ticket: chunk::Ticket,
	) -> Result<chunk::ArctexTicket, AnyError> {
		let arctex_ticket = Arc::new(Mutex::new(ticket));
		self.load_request_sender.try_send(arctex_ticket.clone())?;
		Ok(arctex_ticket)
	}
}
