use crate::app;
use engine::ui::oui::{
	viewport::Viewport,
	widget::{ArcLockWidget, Widget},
};
use std::sync::{Arc, RwLock};

pub struct AppStateViewport {
	root_widget: Option<ArcLockWidget>,
}

impl AppStateViewport {
	pub fn new() -> Self {
		Self { root_widget: None }
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	pub fn with_root(mut self, widget: ArcLockWidget) -> Self {
		self.set_root(widget);
		self
	}

	pub fn set_root(&mut self, widget: ArcLockWidget) {
		self.root_widget = Some(widget);
	}

	pub fn take_root(&mut self) -> Option<ArcLockWidget> {
		self.root_widget.take()
	}
}

impl Viewport for AppStateViewport {
	fn get_root(&self) -> &Option<ArcLockWidget> {
		&self.root_widget
	}
}

pub trait AppStateView: Widget {
	fn new() -> Self;
}

type PresentationInitializer = Box<dyn Fn() -> ArcLockWidget + Send + Sync>;
type PresentationList = Vec<(app::state::State, PresentationInitializer)>;
impl AppStateViewport {
	/// Returns a mapping of [`application state`](crate::app::state::State) to a ui which should be created
	/// and set as the root of the viewport when the application enters the provided state.
	fn presentation_list() -> PresentationList {
		use app::state::{State, State::*};
		fn wrap_state<TWidget>(state: State) -> (State, PresentationInitializer)
		where
			TWidget: 'static + AppStateView + Send + Sync,
		{
			(state, Box::new(|| TWidget::new().arclocked()))
		}
		vec![
			wrap_state::<crate::ui::launch::Launch>(Launching),
			wrap_state::<crate::ui::home::Home>(MainMenu),
			wrap_state::<crate::ui::loading::Loading>(LoadingWorld),
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
				log::debug!(
					"Adding viewport callback for transition into {:?}",
					presentation.0
				);
				app_state.add_callback(
					OperationKey(None, Some(Enter), Some(presentation.0)),
					move |_operation| {
						log::debug!("Instinating ui root for {:?}", _operation.next());
						if let Ok(mut viewport) = callback_viewport.write() {
							viewport.set_root(ui_instantiator());
						}
					},
				);
			}
		}
	}
}
