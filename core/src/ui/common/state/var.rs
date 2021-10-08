use super::super::state::Flag;
use enumset::EnumSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Var<T> {
	states: Vec<(EnumSet<Flag>, T)>,
	default_state: T,
}

impl<T> Var<T> {
	pub fn new(default_state: T) -> Self {
		Self {
			states: Vec::new(),
			default_state,
		}
	}

	pub fn with(mut self, flags: EnumSet<Flag>, state: T) -> Self {
		self.states.push((flags, state));
		self
	}

	pub fn first(&self, flags: EnumSet<Flag>) -> &T {
		for state in self.states.iter() {
			if flags.is_superset(state.0) {
				return &state.1;
			}
		}
		&self.default_state
	}
}
