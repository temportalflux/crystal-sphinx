use crate::network::storage::server::user;
use engine::{
	network::{self, event, mode, processor::Processor, LocalData},
	utility::VoidResult,
};

pub fn register_bonus_processors(
	builder: &mut network::Builder,
	auth_cache: &user::pending::ArcLockCache,
	active_cache: &user::active::ArcLockCache,
	app_state: &crate::app::state::ArcLockMachine,
) {
	use event::Kind::*;

	for event in [Disconnected, Stop].iter() {
		builder.add_processor(
			event.clone(),
			vec![mode::Kind::Client].into_iter(),
			CloseClient {
				app_state: app_state.clone(),
			},
		);
	}
	builder.add_processor(
		Disconnected,
		mode::all().into_iter(),
		RemoveUser {
			auth_cache: auth_cache.clone(),
			active_cache: active_cache.clone(),
		},
	);
}

#[derive(Clone)]
struct CloseClient {
	app_state: crate::app::state::ArcLockMachine,
}

impl Processor for CloseClient {
	fn process(
		&self,
		kind: &event::Kind,
		_data: &mut Option<event::Data>,
		_local_data: &LocalData,
	) -> VoidResult {
		use crate::app::state::State::*;
		if *kind == event::Kind::Disconnected || *kind == event::Kind::Stop {
			if let Ok(mut app_state) = self.app_state.write() {
				app_state.transition_to(MainMenu, None);
			}
		}
		Ok(())
	}
}

#[derive(Clone)]
struct RemoveUser {
	auth_cache: user::pending::ArcLockCache,
	active_cache: user::active::ArcLockCache,
}

impl Processor for RemoveUser {
	fn process(
		&self,
		_kind: &event::Kind,
		data: &mut Option<event::Data>,
		_local_data: &LocalData,
	) -> VoidResult {
		if let Some(event::Data::Connection(connection)) = data {
			if let Ok(mut auth_cache) = self.auth_cache.write() {
				let _ = auth_cache.remove(&connection.address);
			}
			if let Ok(mut active_cache) = self.active_cache.write() {
				let _ = active_cache.remove(&connection.address);
			}
		}
		Ok(())
	}
}
