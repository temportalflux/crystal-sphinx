pub mod packet;

pub fn create_builder(app_state: &crate::app::state::ArcLockMachine) -> engine::network::Builder {
	let mut net_builder = engine::network::Builder::default()
		.with_port(25565)
		.with_args();
	packet::register_types(&mut net_builder, &app_state);
	net_builder
}
