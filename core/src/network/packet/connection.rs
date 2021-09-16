use crate::server::user::pending::ArcLockAuthCache;
use engine::{
	network::{
		self, event, mode,
		packet::{DeliveryGuarantee::*, OrderGuarantee::*, Packet},
		processor::Processor,
		LocalData, Network,
	},
	utility::VoidResult,
};

pub fn register_bonus_processors(builder: &mut network::Builder, auth_cache: &ArcLockAuthCache) {
	use event::Kind::*;
	builder.add_processor(
		Disconnected,
		mode::all().into_iter(),
		RemovePendingUser {
			auth_cache: auth_cache.clone(),
		},
	);
}

#[derive(Clone)]
struct RemovePendingUser {
	auth_cache: ArcLockAuthCache,
}

impl Processor for RemovePendingUser {
	fn process(
		&self,
		_kind: &event::Kind,
		data: &mut Option<event::Data>,
		_local_data: &LocalData,
	) -> VoidResult {
		if let Some(event::Data::Connection(connection)) = data {
			if let Ok(mut auth_cache) = self.auth_cache.write() {
				let _ = auth_cache.remove_pending_user(&connection.address);
			}
		}
		Ok(())
	}
}
