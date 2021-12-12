mod debug_window;
pub use debug_window::*;

mod network_connect;
pub use network_connect::*;
mod network_load;
pub use network_load::*;
mod network_stop;
pub use network_stop::*;

mod world_load;
pub use world_load::*;
mod world_unload;
pub use world_unload::*;

mod command;
pub use command::*;

use std::sync::{Arc, Mutex, RwLock};
pub fn create_list(app_state: &Arc<RwLock<crate::app::state::Machine>>) -> CommandList {
	let mut cmds: Vec<ArctexCommand> = vec![];
	cmds.push(LoadNetwork::new(app_state.clone()).as_arctex());
	cmds.push(UnloadNetwork::new(app_state.clone()).as_arctex());
	cmds.push(Connect::new(app_state.clone()).as_arctex());
	Arc::new(Mutex::new(cmds))
}
