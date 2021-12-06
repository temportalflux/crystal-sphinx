mod debug_window;
pub use debug_window::*;

mod load_world;
pub use load_world::*;

mod unload_world;
pub use unload_world::*;

mod command;
pub use command::*;

use std::sync::{Arc, Mutex, RwLock};
pub fn create_list(app_state: &Arc<RwLock<crate::app::state::Machine>>) -> CommandList {
	let mut cmds: Vec<ArctexCommand> = vec![];
	cmds.push(LoadWorld::new(app_state.clone()).as_arctex());
	cmds.push(UnloadWorld::new(app_state.clone()).as_arctex());
	Arc::new(Mutex::new(cmds))
}
