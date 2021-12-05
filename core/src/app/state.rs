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

pub type TransitionData = Option<Box<dyn std::any::Any + Send + Sync>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationKey(pub Option<State>, pub Option<Transition>, pub Option<State>);
pub struct Operation<'transition>(State, Transition, State, &'transition TransitionData);
pub type FnOperation = Box<dyn Fn(&Operation) + Send + Sync>;

impl<'transition> Operation<'transition> {
	pub fn prev(&self) -> &State {
		&self.0
	}

	pub fn transition(&self) -> &Transition {
		&self.1
	}

	pub fn next(&self) -> &State {
		&self.2
	}

	pub fn data(&self) -> &'transition TransitionData {
		&self.3
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
	pending_transition: Option<State>,
	next_transition: Option<(State, TransitionData)>,
}

impl Machine {
	pub fn new(state: State) -> Self {
		Self {
			state,
			callbacks: HashMap::new(),
			pending_transition: None,
			next_transition: None,
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	pub fn get(&self) -> State {
		self.state
	}

	pub fn has_next_transition(&self) -> bool {
		self.next_transition.is_some()
	}

	pub fn transition_to(&mut self, next_state: State, data: TransitionData) {
		assert!(!self.has_next_transition());
		if self.pending_transition.is_some() {
			log::info!(target: "app-state", "Enqueuing transition to {:?} for after the current transition", next_state);
			self.next_transition = Some((next_state, data));
		} else {
			self.perform_transition((next_state, data));
		}
	}

	fn perform_transition(&mut self, transition: (State, TransitionData)) {
		assert!(self.pending_transition.is_none());
		let (next_state, data) = transition;
		self.pending_transition = Some(next_state);

		let prev_state = self.state;
		log::info!(target: "app-state", "Transitioning from {:?} to {:?}", prev_state, next_state);
		self.dispatch_callback(Operation(prev_state, Transition::Exit, next_state, &data));
		self.state = next_state;
		self.dispatch_callback(Operation(prev_state, Transition::Enter, next_state, &data));

		let _completed_transition = self.pending_transition.take();

		if let Some(next_transition) = self.next_transition.take() {
			self.perform_transition(next_transition);
		}
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

	fn dispatch_callback(&mut self, operation: Operation) {
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
