use engine::network::mode;

#[derive(Clone)]
pub struct Instruction {
	pub name: String,
	pub mode: mode::Set,
}
