use std::sync::{Arc, Mutex};

pub type CommandList = Arc<Mutex<Vec<ArctexCommand>>>;
pub type ArctexCommand = Arc<Mutex<dyn Command + 'static>>;
pub trait Command {
	fn is_allowed(&self) -> bool;
	fn render(&mut self, ui: &mut egui::Ui);
	fn as_arctex(self) -> ArctexCommand
	where
		Self: Sized + 'static,
	{
		Arc::new(Mutex::new(self))
	}
}
