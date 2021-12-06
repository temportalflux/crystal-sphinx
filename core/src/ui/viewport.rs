use crate::app;
use engine::ui::oui::{viewport::Viewport, widget::ArcLockWidget};
use std::sync::{Arc, RwLock, Weak};

pub struct AppStateViewport {
	root_widget: Option<ArcLockWidget>,
	system: Option<Weak<RwLock<engine::ui::System>>>,
}

impl AppStateViewport {
	pub fn new() -> Self {
		Self {
			root_widget: None,
			system: None,
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	fn set_root(&mut self, widget: ArcLockWidget) {
		self.load_image_ids_for(&widget);
		self.root_widget = Some(widget);
	}

	pub fn set_system(&mut self, system: &Arc<RwLock<engine::ui::System>>) {
		self.system = Some(Arc::downgrade(&system));
		if let Some(widget) = self.root_widget.as_ref() {
			self.load_image_ids_for(widget);
		}
	}

	fn load_image_ids_for(&self, widget: &ArcLockWidget) {
		if let Some(weak_system) = self.system.as_ref() {
			if let Some(arc_system) = weak_system.upgrade() {
				if let Ok(mut system) = arc_system.write() {
					let ids = widget.read().unwrap().get_image_ids();
					for id in ids.into_iter() {
						let _ = system.add_texture(&id);
					}
				}
			}
		}
	}
}

impl Viewport for AppStateViewport {
	fn get_root(&self) -> &Option<ArcLockWidget> {
		&self.root_widget
	}
}

macro_rules! init_view_state {
	($state_id:expr, $class_id:expr) => {
		($state_id, Box::new(|| Arc::new(RwLock::new($class_id))))
	};
}
type PresentationInitializer = Box<dyn Fn() -> ArcLockWidget + Send + Sync>;
type PresentationList = Vec<(app::state::State, PresentationInitializer)>;
impl AppStateViewport {
	/// Returns a mapping of [`application state`](crate::app::state::State) to a ui which should be created
	/// and set as the root of the viewport when the application enters the provided state.
	fn presentation_list() -> PresentationList {
		use crate::ui::{home::Home, hud::Hud, launch::Launch, loading::Loading};
		use app::state::State::*;
		vec![
			init_view_state!(Launching, Launch::new()),
			init_view_state!(MainMenu, Home::new()),
			init_view_state!(LoadingWorld, Loading::new()),
			init_view_state!(InGame, Hud::new()),
			init_view_state!(Unloading, Loading::new()),
		]
	}

	pub fn add_state_listener(
		viewport: &Arc<RwLock<AppStateViewport>>,
		app_state: &Arc<RwLock<app::state::Machine>>,
	) {
		use app::state::{Transition::*, *};
		if let Ok(mut app_state) = app_state.write() {
			for presentation in Self::presentation_list().into_iter() {
				let callback_viewport = viewport.clone();
				let ui_instantiator = presentation.1;
				app_state.add_callback(
					OperationKey(None, Some(Enter), Some(presentation.0)),
					move |_operation| {
						if let Ok(mut viewport) = callback_viewport.write() {
							viewport.set_root(ui_instantiator());
						}
					},
				);
			}
		}
	}
}
