use engine::ui::egui::Element;

/// In-Game debug window for examining information about a chunk in the world.
pub struct ChunkInspector {
	is_open: bool,
}

impl ChunkInspector {
	pub fn new() -> Self {
		Self { is_open: false }
	}
}

impl super::PanelWindow for ChunkInspector {
	fn is_open_mut(&mut self) -> &mut bool {
		&mut self.is_open
	}
}

impl Element for ChunkInspector {
	fn render(&mut self, ctx: &egui::Context) {
		if !self.is_open {
			return;
		}
		egui::Window::new("Chunk Inspector")
			.open(&mut self.is_open)
			.show(ctx, move |_ui| {});
	}
}
