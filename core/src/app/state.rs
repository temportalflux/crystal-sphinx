use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
	/// In the out-of-game main menus.
	MainMenu,
	/// Loading or creating a local world.
	LoadingWorld,
	/// The network is connecting and waiting for world data from a server.
	Connecting,
	/// Player is in the world and can move at will.
	InGame,
	/// Player is disconnecting from (remote) or closing (local) a world.
	Unloading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
	Enter,
	Exit,
}

impl Transition {
	pub fn all() -> Vec<Transition> {
		vec![Self::Enter, Self::Exit]
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationKey(pub Option<State>, pub Option<Transition>, pub Option<State>);
pub struct Operation(State, Transition, State);
pub type FnOperation = Box<dyn Fn(&Operation) + Send + Sync>;

impl Operation {
	pub fn prev(&self) -> &State {
		&self.0
	}

	pub fn transition(&self) -> &Transition {
		&self.1
	}

	pub fn next(&self) -> &State {
		&self.2
	}

	fn all_keys(&self) -> Vec<OperationKey> {
		let mut keys = Vec::new();
		for transition in [None, Some(*self.transition())] {
			for prev in [None, Some(*self.prev())] {
				for next in [None, Some(*self.next())] {
					keys.push(OperationKey(prev, transition, next));
				}
			}
		}
		keys
	}
}

pub struct Machine {
	state: State,
	callbacks: HashMap<OperationKey, Vec<FnOperation>>,
}

impl Machine {
	pub fn new(state: State) -> Self {
		Self {
			state,
			callbacks: HashMap::new(),
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	pub fn transition_to(&mut self, next_state: State) {
		let prev_state = self.state;
		self.dispatch_callback(Operation(prev_state, Transition::Exit, next_state));
		self.state = next_state;
		self.dispatch_callback(Operation(prev_state, Transition::Enter, next_state));
	}

	pub fn add_callback<F>(&mut self, key: OperationKey, callback: F)
	where
		F: Fn(&Operation) + Send + Sync + 'static,
	{
		if !self.callbacks.contains_key(&key) {
			self.callbacks.insert(key.clone(), Vec::new());
		}
		self.callbacks
			.get_mut(&key)
			.unwrap()
			.push(Box::new(callback));
	}

	fn dispatch_callback(&self, operation: Operation) {
		let relevant_callbacks = operation
			.all_keys()
			.into_iter()
			.filter_map(|key| self.callbacks.get(&key))
			.flatten();
		for callback in relevant_callbacks {
			callback(&operation);
		}
	}
}
