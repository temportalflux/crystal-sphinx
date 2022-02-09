use engine::{utility::Result, Engine, EngineSystem};
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

pub mod storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
	// Application is loading assets and systems.
	Launching,

	/// In the out-of-game main menus.
	MainMenu,

	/// Loading or creating a local world (always a server, can also be a client).
	LoadingWorld,
	/// Player is closing a local world (always a server, can also be a client).
	Unloading,

	/// The network is connecting and waiting for world data from a server.
	Connecting,
	// Player is disconnecting from (remote) a server-world (aka network is stopping).
	Disconnecting,

	/// World is active.
	/// Can be on a dedicated server, dedicated client, or integrated client-server.
	InGame,
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
pub struct Operation<'transition>(
	Option<State>,
	Transition,
	State,
	&'transition TransitionData,
);
pub type FnOperation = Box<dyn Fn(&Operation) + Send + Sync>;

impl<'transition> Operation<'transition> {
	pub fn prev(&self) -> &Option<State> {
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
		let mut prev_list = vec![None];
		if self.prev().is_some() {
			prev_list.push(*self.prev());
		}
		for prev in prev_list.into_iter() {
			for transition in [None, Some(*self.transition())] {
				for next in [None, Some(*self.next())] {
					keys.push(OperationKey(prev, transition, next));
				}
			}
		}
		keys
	}
}

pub type ArcLockMachine = Arc<RwLock<Machine>>;
pub struct Machine {
	state: State,
	callbacks: HashMap<OperationKey, Vec<FnOperation>>,
	next_transition: Option<(State, TransitionData)>,
}

impl Machine {
	fn new(state: State) -> Self {
		Self {
			state,
			callbacks: HashMap::new(),
			next_transition: None,
		}
	}

	fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	pub fn create(state: State, engine: &mut Engine) -> Arc<RwLock<Self>> {
		let machine = Self::new(state).arclocked();
		engine.add_weak_system(Arc::downgrade(&machine));
		machine
	}

	pub fn get(&self) -> State {
		self.state
	}

	pub fn has_next_transition(&self) -> bool {
		self.next_transition.is_some()
	}

	pub fn transition_to(&mut self, next_state: State, data: TransitionData) {
		assert!(!self.has_next_transition());
		profiling::scope!("transition_to", &format!("{:?}", next_state));
		log::info!(target: "app-state", "Enqueuing next state {:?}", next_state);
		self.next_transition = Some((next_state, data));
	}

	fn perform_transition(&mut self, transition: (State, TransitionData)) {
		profiling::scope!(
			"perform_transition",
			&format!("{:?} -> {:?}", self.state, transition.0)
		);
		let (next_state, data) = transition;

		let prev_state = self.state;
		log::info!(target: "app-state", "Transition: {:?} -> {:?}", prev_state, next_state);
		self.dispatch_callback(Operation(
			Some(prev_state),
			Transition::Exit,
			next_state,
			&data,
		));
		self.state = next_state;
		self.dispatch_callback(Operation(
			Some(prev_state),
			Transition::Enter,
			next_state,
			&data,
		));
	}

	pub fn add_callback<F>(&mut self, key: OperationKey, callback: F)
	where
		F: Fn(&Operation) + Send + Sync + 'static,
	{
		if key.2 == Some(self.state) && key.1 == Some(Transition::Enter) {
			callback(&Operation(None, Transition::Enter, self.state, &None));
		}

		if !self.callbacks.contains_key(&key) {
			self.callbacks.insert(key.clone(), Vec::new());
		}
		self.callbacks
			.get_mut(&key)
			.unwrap()
			.push(Box::new(callback));
	}

	pub fn add_async_callback<F, T>(&mut self, key: OperationKey, callback: F)
	where
		F: Fn(&Operation) -> T + Send + Sync + 'static,
		T: futures::future::Future<Output = Result<()>> + Send + 'static,
	{
		self.add_callback(key, move |operation| {
			engine::task::spawn("app-state".to_string(), callback(operation));
		});
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

impl EngineSystem for Machine {
	fn update(&mut self, _delta_time: std::time::Duration, _has_focus: bool) {
		if let Some(transition) = self.next_transition.take() {
			self.perform_transition(transition);
		}
	}
}
