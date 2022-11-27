use crate::{common::world, entity::system::replicator::relevancy::Relevance};
use engine::{channels::broadcast, utility::ValueSet};
use std::sync::{Arc, Weak};

pub mod chunk;

pub struct ChunkChannel {
	channel: chunk::OperationPair,
	/// Handle which keeps the async-task alive as long as this struct is not dropped.
	#[allow(dead_code)]
	task_handle: Arc<()>,
}

impl ChunkChannel {
	pub fn new(
		mut recv_updates: broadcast::BusReader<world::UpdateBlockId>,
		systems: Weak<ValueSet>,
	) -> Self {
		static LOG: &'static str = "client_chunk_channel";
		let channel = engine::channels::mpsc::unbounded();
		let task_handle = Arc::new(());

		let chunk_sender = channel.0.clone();
		let weak_handle = Arc::downgrade(&task_handle);
		engine::task::spawn(LOG.to_owned(), async move {
			while weak_handle.strong_count() > 0 {
				if let Ok(update) = recv_updates.try_recv() {
					match update {
						world::Update::Inserted(coord, contents) => {
							let is_relevant = {
								let Some(systems) = systems.upgrade() else { continue; };
								let Some(arc_relevance) = systems.get_arclock::<Relevance>() else { continue; };
								let relevance = arc_relevance.read().unwrap();
								relevance.is_relevant(&coord)
							};
							
							if is_relevant {
								let _ = chunk_sender
									.try_send(chunk::Operation::Insert(coord, contents.to_vec()));
							}
						}
						world::Update::Dropped(coord, _items) => {
							let _ = chunk_sender.try_send(chunk::Operation::Remove(coord));
						}
					}
				}
			}
			Ok(())
		});
		Self {
			channel,
			task_handle,
		}
	}

	pub fn send(&self) -> &chunk::OperationSender {
		&self.channel.0
	}

	pub fn recv(&self) -> &chunk::OperationReceiver {
		&self.channel.1
	}
}
