use egui_winit::winit;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("event loop failed: {0}")]
	EventLoopFail(#[source] winit::error::EventLoopError),

	#[error("could not build event loop: {0}")]
	EventLoopBuildFail(#[source] winit::error::EventLoopError),

	#[error("could not send message over channel: channel disconnected")]
	SendingOverChannel,

	#[error("error of unexpected type reached root error channel: {0:?}")]
	UnknownErrorFromRoot(Box<dyn std::fmt::Debug + Sync + Send + 'static>),

	#[cfg(feature = "ctrlc")]
	#[error("could not set ctrlc handler: {0}")]
	SetSigHandler(#[source] ctrlc::Error),
}
