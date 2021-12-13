use engine::network::mode;
use crate::app::state::State;

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

impl Instruction {
	pub fn get_next_app_state(&self) -> Option<State> {
		match &self.directive {
			Directive::LoadWorld(_) => Some(State::InGame),
			Directive::Connect(_) => None,
		}
	}
}
