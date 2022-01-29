use engine::network::mode;

#[derive(Clone)]
pub struct Instruction {
	pub mode: mode::Set,
	pub port: Option<u16>,
	pub world_name: Option<String>,
	pub server_url: Option<String>,
}
