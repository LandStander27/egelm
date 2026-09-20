//! Integrations with XDG desktop portals.

mod file_dialog;
mod open_uri;
mod shortcuts;

pub use file_dialog::{OpenFile, SaveFile};
pub use open_uri::OpenURI;
pub use shortcuts::GlobalShortcuts;

/// An asynchronous request sent to an XDG desktop portal.
#[async_trait::async_trait]
pub trait PortalRequest {
	/// The result produced when the portal request successfully resolves.
	type Output;

	/// Sends the request to the desktop portal, optionally associated with a parent window.
	async fn send(self, window: Option<ashpd::WindowIdentifier>) -> Result<Self::Output, crate::error::Error>;
}
