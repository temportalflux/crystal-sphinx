use engine::network::mode;

#[derive(Clone)]
pub struct Instruction {
	pub mode: mode::Set,
	pub port: Option<u16>,
	pub directive: Directive,
}

#[derive(Clone)]
pub enum Directive {
	LoadWorld(/*world name*/ String),
	Connect(/*url*/ String),
}
