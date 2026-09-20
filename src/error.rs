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

	#[cfg(feature = "storage")]
	/// Failed to parse a value from storage.
	#[error("could not parse value from storage: {0}")]
	StorageParsing(#[source] ron::error::SpannedError),

	#[cfg(feature = "storage")]
	/// Failed to serialize a value to storage.
	#[error("could not serialize value to storage: {0}")]
	StorageSerialization(#[source] ron::error::Error),

	#[cfg(feature = "storage")]
	/// Expected UTF-8 encoding but encountered invalid bytes.
	#[error("invalid utf8: {0}")]
	ExpectedUtf8(#[source] std::string::FromUtf8Error),

	#[cfg(feature = "storage")]
	/// An I/O error occurred while writing to storage.
	#[error("could not perform write: {0}")]
	WriteFailure(#[source] std::io::Error),

	#[cfg(any(feature = "storage", feature = "portal"))]
	/// An I/O error occurred while reading from disk.
	#[error("could not perform read: {0}")]
	ReadFailure(#[source] std::io::Error),

	#[cfg(feature = "portal")]
	/// A ashpd error when trying to use the XDG Desktop Portal.
	#[error("portal error: {0}")]
	Portal(#[source] ashpd::Error),
}
