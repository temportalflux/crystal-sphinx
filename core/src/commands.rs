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

// TODO: Add a command to disassociate the player.
// This would "save" a players location at the time of cheat usage,
// causing only the camera to update to their new position.
// Effect:
// - ChunkLoader tickets are bound to the user's last location
// - Any future physics are bound to the last location
// - Player body render stays put
// - Only the camera moves
// This will let me look around the world (what chunks are rendered, what entities are far away, etc),
// without causing world updates.
