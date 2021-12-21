use crate::app::state::State;
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

impl Instruction {
	pub fn get_next_app_state(&self) -> Option<State> {
		match self.directive {
			Directive::LoadWorld(_) => {
				// If this is a dedicated server, then loading the world
				// should automatically transition to the InGame state.
				if self.mode == mode::Kind::Server {
					Some(State::InGame)
				}
				// Otherwise, CotoS will go through the auth flow anyhow,
				// and transition to InGame like normal Clients do.
				else {
					None
				}
			}
			Directive::Connect(_) => None,
		}
	}
}
