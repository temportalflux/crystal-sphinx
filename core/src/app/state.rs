use anyhow::Result;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

pub mod storage;

#[cfg_attr(doc, aquamarine::aquamarine)]
/// The life-cycle phase of the application.
///
/// [Edit diagram](https://mermaid.live/edit/#pako:eNp9VE1r3DAQ_SuDThtI8H0PgZJdmoWEFtLSg-yDak1jEa9kLDltkP3fK2nWtjab9iLe6L0ZzZftWW0ksi177kXXwLcdlLp0XzrUn7qO83C0qhZOGQ3xEmVVRcGDGHTdKP284QusriLzKJR-RD1seEQQIREPRsig-mH6VgYvsiCZJLgzWmPtUtAVE3fQn8URN_ygIQK6_K5birLhCyRmp2ydRTszSbH_oxzn8aSCsqrh5uYWlqrOqk3UV2NTMUHqDxZ2KGOHUMIT9q_YT9Ej00SfUZsR5s58wMP4hnaEvEVBlXUzpeii8q5VqJMvL-5DFBDwO-qLatVcxnl3k-nSlfcRgmuQgk3TKqLW_y-Z07B4cQLgTMjKpm7kaa1DXe_uhZa2ES_o_d468bNVtjm5QjNzH6ez7gVNDMUrRosXCdJq0fsLl4ahLEgamoQ6VTDC2YpcrFB6IC__IiClPMKyiGcLmvzJWkaQmd6TQSlTtRn9LumwafNrcX__IZ4Ly7OeMayzS99BEc_0YRVV4iguu2ZH7I9CyfB_8KUGKFlYkdB9tg1Qiv6lZKWegm7oZEhsL5UzPdv-Eq3FayYGZ57edM22rh9wFu2UCP-a40k1_QXN7qN3)
/// ```mermaid
/// graph TD
/// 	OpenApp[[Application Opened]]
/// 	Launching([Launching])
/// 	MainMenu([Main Menu])
/// 	LoadingWorld([Loading World])
/// 	Connecting([Connecting])
/// 	InGame([In Game])
/// 	Unloading([Unloading])
/// 	Disconnecting([Disconnecting])
/// 	Exit[[Exit]]
///
/// 	OpenApp --> Launching
/// 	Launching --> PostLoadApp{Is Dedicated Server}
/// 	PostLoadApp -->|no| MainMenu
/// 	PostLoadApp --> |yes| LoadingWorld
///
/// 	MainMenu
/// 		--> ClientLoad[/Host a world/]
/// 		--> LoadingWorld
///
/// 	LoadingWorld
/// 		--> LoadWorld{{Load the world}}
/// 		--> InGame
/// 	MainMenu
/// 		--> ClientConnect[/Connect to a server/]
/// 		--> Connecting
/// 		--> Handshake{{Establish server handshake}}
/// 		--> InGame
///
/// 	InGame --> LeaveGame[/Leave World/]
/// 	LeaveGame -->|is dedicatd client| Disconnecting
/// 	Disconnecting --> MainMenu
/// 	LeaveGame -->|is server| Unloading
/// 	Unloading --> UnloadWorld
/// 	UnloadWorld{{Unload World}}
/// 	UnloadWorld -->|is dedicated server| Exit
/// 	UnloadWorld -->|is client| MainMenu
/// 	MainMenu --> ClientExit[/Exit Game/] --> Exit
/// ```
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
	pub fn new(state: State) -> Self {
		Self {
			state,
			callbacks: HashMap::new(),
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

	pub fn clear_callbacks(&mut self) {
		self.callbacks.clear();
	}

	pub fn update(&mut self) {
		if let Some(transition) = self.next_transition.take() {
			self.perform_transition(transition);
		}
	}
}
