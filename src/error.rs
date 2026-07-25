//! Errors returned by application and event-loop operations.

use egui_winit::winit;

/// An error that can prevent an [`crate::window::App`] from running.
///
/// These errors cover event-loop creation and execution, internal message
/// delivery, root error routing, and optional Ctrl-C handler installation.
///
/// # Examples
///
/// ```
/// use egelm::error::Error;
///
/// let error = Error::SendingOverChannel;
/// assert_eq!(error.to_string(), "could not send message over channel: channel disconnected");
/// ```
#[derive(thiserror::Error, Debug)]
pub enum Error {
	/// The platform event loop stopped with an error.
	#[error("event loop failed: {0}")]
	EventLoopFail(#[source] winit::error::EventLoopError),

	/// The platform event loop could not be built.
	#[error("could not build event loop: {0}")]
	EventLoopBuildFail(#[source] winit::error::EventLoopError),

	/// An internal channel was disconnected before a message could be sent.
	#[error("could not send message over channel: channel disconnected")]
	SendingOverChannel,

	/// The requested renderer was not enabled at compile time.
	#[error("the {0} renderer is not enabled")]
	RendererUnavailable(&'static str),

	#[cfg(all(feature = "ctrlc", not(target_os = "android")))]
	/// The process Ctrl-C handler could not be installed.
	#[error("could not set ctrlc handler: {0}")]
	SetSigHandler(#[source] ctrlc::Error),
}
