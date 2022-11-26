use crate::common::utility::ThreadHandle;
use crate::common::world::Database;
use crate::server::world::chunk::{
	self,
	ticket::{self, Ticket},
	Chunk, Level,
};
use anyhow::Result;
use engine::{math::nalgebra::Point3, utility::spawn_thread};
use std::sync::RwLock;
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, Weak},
};

/// The log category for the chunk loading thread.
static LOG: &'static str = "chunk-loading";

/// State data about the loading thread.
pub(crate) struct ThreadState {
	root_dir: PathBuf,
	database: Arc<RwLock<Database>>,

	/// List of inactive and recently dropped tickets (and the chunk coordinates they reference).
	ticket_bindings: Vec<(Weak<Ticket>, Vec<Point3<i64>>)>,
	/// Map of coordinate to chunk states (and the actual strong reference to keep the chunk loaded).
	chunk_states: HashMap<Point3<i64>, ChunkState>,

	/// The amount of time a chunk can spend in `ticketless_chunks` before being saved to disk and dropped.
	expiration_delay: std::time::Duration,
	/// The earliest time in `ticketless_chunks`. Will be None if there are no chunks waiting to be unloaded.
	earliest_expiration_timestamp: Option<std::time::Instant>,
	/// The list of chunk coordinates without tickets, paired with the time the coordinate was added to the list.
	/// If they remain in this list beyond `expiration_delay`, and have not been associated with
	/// a new ticket in that time, they are unloaded (saved to disk and dropped from memory).
	ticketless_chunks: Vec<(std::time::Instant, Point3<i64>)>,

	/// Marked as true if/when the sender of the channel has been dropped.
	disconnected_from_requests: bool,
}

/// Begins the chunk loading thread, returning its handle.
/// If the handle is dropped, the thread will stop at the next loop.
pub fn start(
	root_dir: PathBuf,
	incoming_requests: ticket::Receiver,
	database: Weak<RwLock<Database>>,
) -> anyhow::Result<ThreadHandle> {
	let handle = Arc::new(());
	let weak_handle = Arc::downgrade(&handle);
	let database = database.clone();
	let root_dir = root_dir.clone();
	let join_handle = spawn_thread(LOG, move || -> Result<()> {
		let mut thread_state = ThreadState {
			root_dir: root_dir.clone(),
			database: database.upgrade().unwrap(),
			ticket_bindings: Vec::new(),
			chunk_states: HashMap::new(),
			expiration_delay: std::time::Duration::from_secs(60),
			earliest_expiration_timestamp: None,
			ticketless_chunks: Vec::new(),
			disconnected_from_requests: false,
		};

		// while the database has not been discarded,
		// processing any pending load requests & unload any chunks no longer needed
		log::info!(target: LOG, "Starting chunk-loading thread");
		while weak_handle.strong_count() > 0 {
			thread_state.update(&incoming_requests);
			std::thread::sleep(std::time::Duration::from_millis(1));
		}
		log::info!(target: LOG, "Ending chunk-loading thread");

		Ok(())
	})?;

	Ok(ThreadHandle::new(handle, join_handle))
}

impl ThreadState {
	#[profiling::function]
	fn update(&mut self, incoming_requests: &ticket::Receiver) {
		self.process_new_tickets(&incoming_requests);
		self.update_dropped_tickets();
		if self.has_expired_chunks() {
			let chunks_for_unloading = self.find_expired_chunks();
			self.unload_expired_chunks(chunks_for_unloading);
		}
	}

	#[profiling::function]
	fn process_new_tickets(&mut self, incoming_requests: &ticket::Receiver) {
		use engine::channels::mpsc::TryRecvError;
		let mut has_emptied_requests = false;
		while !self.disconnected_from_requests && !has_emptied_requests {
			match incoming_requests.try_recv() {
				Ok(weak_ticket) => {
					// TODO: Multiple chunks could be loaded concurrently.
					// If requests are gathered first and then all new chunks are loaded at once,
					// we could increase the throughput of the chunk loader.
					self.sync_process_ticket(weak_ticket);
				}
				// no events, continue the loop after a short nap
				Err(TryRecvError::Empty) => {
					has_emptied_requests = true;
				}
				// If disconnected, then kill the thread
				Err(TryRecvError::Disconnected) => {
					log::debug!(target: "chunk-loading", "Disconnected from chunk request channel");
					self.disconnected_from_requests = true;
				}
			}
		}
	}

	#[profiling::function]
	fn sync_process_ticket(&mut self, weak_ticket: Weak<Ticket>) {
		let arc_ticket = match weak_ticket.upgrade() {
			Some(ticket) => ticket,
			None => return, // early out if the user has already dropped the ticket
		};
		let processed_chunks = self.sync_load_ticket_chunks(arc_ticket);
		let mut ticket_chunks = Vec::with_capacity(processed_chunks.len());
		for (coordinate, weak_chunk, level) in processed_chunks.into_iter() {
			self.insert_or_update_chunk_state(&weak_ticket, coordinate, level, weak_chunk);
			ticket_chunks.push(coordinate);
		}
		self.ticket_bindings.push((weak_ticket, ticket_chunks));
	}

	#[profiling::function]
	fn sync_load_ticket_chunks(
		&mut self,
		ticket: Arc<Ticket>,
	) -> Vec<(Point3<i64>, Weak<RwLock<chunk::Chunk>>, Level)> {
		let mut chunks = Vec::new();
		let coordinate_levels = ticket.coordinate_levels();
		for (coordinate, level) in coordinate_levels.into_iter() {
			let chunk_id = format!(
				"<{}, {}, {}> @ {:?}",
				coordinate[0], coordinate[1], coordinate[2], level
			);
			profiling::scope!("load-chunk", chunk_id.as_str());

			let weak_chunk = self.sync_load_chunk(coordinate, level);
			chunks.push((coordinate, weak_chunk, level));
		}
		chunks
	}

	fn sync_load_chunk(
		&mut self,
		coordinate: Point3<i64>,
		level: Level,
	) -> Weak<RwLock<chunk::Chunk>> {
		let loaded_chunk = self
			.database
			.read()
			.unwrap()
			.find_chunk(&coordinate)
			.map(|arc| arc.unwrap_server().clone());
		let (_freshly_loaded, arc_chunk) = match loaded_chunk {
			Some(arc_chunk) => (false, Arc::downgrade(&arc_chunk)),
			None => {
				let root_dir = self.root_dir.clone();
				let arc_chunk = Chunk::load_or_generate(&coordinate, level, root_dir);
				let weak_chunk = Arc::downgrade(&arc_chunk);
				let mut database = self.database.write().unwrap();
				database.insert_chunk(coordinate, arc_chunk);
				(true, weak_chunk)
			}
		};

		arc_chunk
	}

	fn insert_or_update_chunk_state(
		&mut self,
		weak_ticket: &Weak<Ticket>,
		coordinate: Point3<i64>,
		level: Level,
		weak_chunk: Weak<RwLock<chunk::Chunk>>,
	) {
		match self.chunk_states.get_mut(&coordinate) {
			Some(state) => {
				if state.level < level {
					state.level = level;
				}
				state.tickets.push(weak_ticket.clone());
			}
			None => {
				self.chunk_states.insert(
					coordinate,
					ChunkState {
						chunk: weak_chunk,
						level: level,
						tickets: vec![weak_ticket.clone()],
					},
				);
			}
		}
	}

	/// Iterate over all bound tickets to detect if any have been dropped.
	/// All chunks related to a dropped ticket, which aren't referenced by other tickets,
	/// are sent to the pending_unload list.
	#[profiling::function]
	fn update_dropped_tickets(&mut self) {
		let now = std::time::Instant::now();
		// Can use `Vec::drain_filter` when that api stabilizes.
		// O(n) performance where `n` is the number of loaded chunks
		let mut i = 0;
		while i < self.ticket_bindings.len() {
			// if there are no strong references, the ticket has been dropped
			if self.ticket_bindings[i].0.strong_count() == 0 {
				// dropped tickets mean their chunks should be moved to a pending list of chunks that will be removed soon
				let (_dropped_ticket, chunks) = self.ticket_bindings.remove(i);
				// Iterate over all the chunks that the dropped ticket referenced
				for coordinate in chunks {
					let state = self.chunk_states.get_mut(&coordinate).unwrap();
					// If the chunk state indicates that no other tickets requested this chunk,
					// then move the chunk to the list of chunks to be unloaded in the near future.
					// This list is always sorted by timestamp such that the earliest are first.
					if state.update() {
						if self.earliest_expiration_timestamp.is_none() {
							self.earliest_expiration_timestamp = Some(now);
						}
						self.ticketless_chunks.push((now, coordinate));
					}
				}
			} else {
				i += 1;
			}
		}
	}

	fn has_expired_chunks(&self) -> bool {
		match self.earliest_expiration_timestamp {
			Some(insertion_time) => {
				std::time::Instant::now().duration_since(insertion_time) > self.expiration_delay
			}
			None => false,
		}
	}

	#[profiling::function]
	fn find_expired_chunks(&mut self) -> Vec<Point3<i64>> {
		// Invalidate the flag indicating that there is some chunk pending expiration.
		// We will later give it a proper value if there are still chunks in the list which will expire later.
		self.earliest_expiration_timestamp = None;

		let mut chunks_for_unloading = Vec::new();
		let now = std::time::Instant::now();
		// Can use `Vec::drain_filter` when that api stabilizes.
		// O(n) performance where `n` is the number of loaded chunks
		let mut i = 0;
		while i < self.ticketless_chunks.len() {
			let (insertion_time, coordinate) = self.ticketless_chunks[i].clone();
			// A chunk has expired if the amount of time since insertion exceeds the maximum.
			let has_expired = now.duration_since(insertion_time) > self.expiration_delay;
			// If the chunk has any new tickets which reference it, it shouldnt be dropped.
			let has_been_renewed = if let Some(state) = self.chunk_states.get(&coordinate) {
				state.tickets.len() > 0
			} else {
				false
			};
			// If any given chunk /should be dropped/ or a new ticket has referenced it, it is no longer "pending" unload.
			if has_expired || has_been_renewed {
				self.ticketless_chunks.remove(i);
				// Only move the chunk to the list to be unloaded if no other tickets reference it.
				if !has_been_renewed {
					let _ = self.chunk_states.remove(&coordinate);
					chunks_for_unloading.push(coordinate);
				}
			} else {
				i += 1;
				// There is still an element in the list, so make sure the next time we iterate
				// is the earliest moment of the next expiration (but no earlier).
				if self.earliest_expiration_timestamp.is_none()
					|| insertion_time < self.earliest_expiration_timestamp.unwrap()
				{
					self.earliest_expiration_timestamp = Some(insertion_time);
				}
			}
		}
		chunks_for_unloading
	}

	#[profiling::function]
	fn unload_expired_chunks(&mut self, mut chunks_for_unloading: Vec<Point3<i64>>) {
		// Unload each chunk, dropping them one-after-one after each iteration
		if !chunks_for_unloading.is_empty() {
			log::debug!(
				target: LOG,
				"Unloading {} chunks",
				chunks_for_unloading.len()
			);
			let mut database = self.database.write().unwrap();
			for coordinate in chunks_for_unloading.drain(..) {
				// remove the chunk from the database before unloading it
				if let Some(entry) = database.remove_chunk(&coordinate) {
					let arc_chunk = entry.unwrap_server();
					// unload the chunk:
					// 1. save to disk
					// 2. drop the arc
					let chunk = arc_chunk.read().unwrap();
					chunk.save();
				}
			}
		}
	}
}

/// Data pertaining to the state of the chunk with respect to loading & tickets.
/// Does NOT contain data pertaining to the state of the chunk in the world.
pub struct ChunkState {
	/// The pointer to the actual chunk world data.
	/// If this is dropped, the chunk is discarded (regardless of if its been saved or not).
	pub chunk: Weak<RwLock<chunk::Chunk>>,
	/// The ticking level of the chunk.
	/// If this state changes during [`update`](ChunkState::update), the value in the chunk world data is updated.
	/// This value is driven by finding the highest level in the list of associated tickets.
	/// If this value changes, a copy is applied to the level in the chunk world data.
	pub level: Level,
	/// The list of tickets which keep this chunk loaded.
	pub tickets: Vec<Weak<Ticket>>,
}

impl ChunkState {
	/// Looks at the list of tickets to determine if the chunk this state represents should remain loaded.
	/// The level of the state is updated here, and returning true means that all associated tickets have been dropped.
	pub fn update(&mut self) -> bool {
		let mut i = 0;
		let mut highest_level = None;
		while i < self.tickets.len() {
			match self.tickets[i].upgrade() {
				None => {
					self.tickets.remove(i);
				}
				Some(arc_ticket) => {
					i += 1;
					let ticket_level: Level = arc_ticket.level.into();
					if highest_level.is_none() || ticket_level > highest_level.unwrap() {
						highest_level = Some(ticket_level);
					}
				}
			}
		}
		match highest_level {
			Some(level) => {
				if self.level != level {
					self.level = level;
					if let Some(arc) = self.chunk.upgrade() {
						arc.write().unwrap().level = level;
					}
				}
				false
			}
			None => true,
		}
	}
}
