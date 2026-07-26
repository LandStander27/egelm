//! Build event-driven desktop interfaces with [`egui`].
//!
//! `egelm` supplies a small widget lifecycle, typed message routing, window
//! management, and reusable dialogs on top of `egui` and `winit`. OpenGL is
//! used by default; the optional `wgpu` feature selects the `wgpu` renderer.
//!
//! # Examples
//!
//! ```
//! use egelm::prelude::*;
//!
//! #[derive(Debug)]
//! struct Greeting;
//!
//! impl LeafWidget for Greeting {
//!     fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
//!         ui.label("Hello from egelm");
//!     }
//! }
//! ```

#[cfg(not(any(glow, wgpu)))]
compile_error!("enable either the `glow` or `wgpu` renderer feature");

#[cfg(all(target_os = "android", not(feature = "android")))]
compile_error!("android targets require the `android` feature");

#[cfg(all(linux, not(any(feature = "wayland", feature = "x11"))))]
compile_error!("linux builds require `wayland` or `x11`");

/// Emoji lookup and display types.
#[cfg(feature = "emoji")]
pub mod emoji;
/// Errors produced by application and event-loop operations.
pub mod error;
/// Native rendering and window-system integration.
pub mod native;
/// Convenient re-exports for applications using `egelm`.
pub mod prelude;
/// Widget lifecycles, message routing, and application management.
pub mod window;

/// Reusable widgets supplied by `egelm`.
pub mod widgets {
	pub use crate::window::about_dialog::AboutDialog;
}
