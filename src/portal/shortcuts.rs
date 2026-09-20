use std::pin::Pin;

use crate::error::Error;

use super::PortalRequest;

use ashpd::desktop::{
	CreateSessionOptions, Session,
	global_shortcuts::{BindShortcutsOptions, GlobalShortcuts as AshpdGlobalShortcuts, NewShortcut},
};
use futures_util::{Stream, StreamExt};

/// An active global shortcuts session.
///
/// When dropped, the portal session is automatically closed and the shortcuts are unregistered.
pub struct GlobalShortcutsSession {
	_session: Session<AshpdGlobalShortcuts>,
	stream: Pin<Box<dyn Stream<Item = String> + Send>>,
}

impl GlobalShortcutsSession {
	/// Waits for the next shortcut to be activated and returns its shortcut ID.
	///
	/// Returns `None` if the session was closed or disconnected.
	pub async fn next_activated(&mut self) -> Option<String> {
		self.stream.next().await
	}
}

/// Request to register one or more global desktop shortcuts.
#[derive(Default)]
pub struct GlobalShortcuts {
	shortcuts: Vec<NewShortcut>,
}

impl GlobalShortcuts {
	/// Creates an empty global shortcuts request.
	pub fn new() -> Self {
		Self::default()
	}

	/// Adds a shortcut with an identifier and human-readable description.
	pub fn shortcut(mut self, id: impl Into<String>, description: impl Into<String>) -> Self {
		self.shortcuts.push(NewShortcut::new(id, description));
		self
	}

	/// Adds a shortcut with an identifier, description, and an initial preferred trigger key combination (e.g. `"Control+Alt+F"`).
	///
	/// Find more [here](https://xdg.pages.freedesktop.org/xdg-specs/shortcuts/latest/#specification).
	pub fn shortcut_with_trigger(mut self, id: impl Into<String>, description: impl Into<String>, preferred_trigger: &str) -> Self {
		self.shortcuts
			.push(NewShortcut::new(id, description).preferred_trigger(Some(preferred_trigger)));
		self
	}
}

#[async_trait::async_trait]
impl PortalRequest for GlobalShortcuts {
	type Output = GlobalShortcutsSession;

	async fn send(self, window: Option<ashpd::WindowIdentifier>) -> Result<Self::Output, Error> {
		let proxy = AshpdGlobalShortcuts::new().await.map_err(Error::Portal)?;

		let session = proxy
			.create_session(CreateSessionOptions::default())
			.await
			.map_err(Error::Portal)?;

		proxy
			.bind_shortcuts(&session, &self.shortcuts, window.as_ref(), BindShortcutsOptions::default())
			.await
			.map_err(Error::Portal)?
			.response()
			.map_err(Error::Portal)?;

		tracing::info!("GlobalShortcuts binded");

		let signal_stream = proxy.receive_activated().await.map_err(Error::Portal)?;
		let stream = Box::pin(signal_stream.map(|act| act.shortcut_id().to_string()));

		Ok(GlobalShortcutsSession { _session: session, stream })
	}
}
