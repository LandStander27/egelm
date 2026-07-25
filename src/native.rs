//! Native platform integrations used by `egelm`.

use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

#[cfg(feature = "glow")]
use egui_glow::glow;
#[cfg(feature = "wgpu")]
use egui_wgpu::wgpu;
use egui_winit::winit::{
	self,
	application::ApplicationHandler,
	event::{StartCause, WindowEvent},
	event_loop::{ActiveEventLoop, EventLoopProxy},
};

/// OpenGL rendering and native window control.
#[cfg(feature = "glow")]
pub(crate) mod _glow;

/// `wgpu` rendering and native window control.
#[cfg(feature = "wgpu")]
pub(crate) mod _wgpu;

/// Renderer selection for the app.
///
/// The `Glow` variant uses OpenGL via the `glow` crate, while the `Wgpu` variant uses the `wgpu` crate for rendering.
/// `wgpu` is the default renderer, but `Glow` may be preferred for compatibility with older hardware or when using OpenGL-specific features.
#[derive(Debug, Clone, Copy)]
pub enum Renderer {
	/// OpenGL via the `glow` crate.
	Glow,

	/// `wgpu` via the `wgpu` crate.
	Wgpu,
}

/// Rendering resources and window controls for the current frame.
///
/// A mutable `Frame` is passed to widget rendering methods. It dereferences to
/// [`Handle`], allowing window-control methods such as [`Handle::exit`] to be
/// called directly on it.
pub struct Frame {
	handle: Handle,

	#[cfg(feature = "glow")]
	gl: Option<Arc<glow::Context>>,

	#[cfg(feature = "wgpu")]
	render_state: Option<egui_wgpu::RenderState>,

	window: Arc<winit::window::Window>,
}

impl Frame {
	/// Returns the OpenGL context used to paint the frame.
	///
	/// Returns None if the `glow` feature is not enabled or if the frame was created without an OpenGL context.
	#[cfg(feature = "glow")]
	pub fn gl(&self) -> Option<&Arc<glow::Context>> {
		self.gl.as_ref()
	}

	/// Returns the `wgpu` state used to paint the frame.
	///
	/// Returns None if the `wgpu` feature is not enabled or if the frame was created without a `wgpu` context.
	#[cfg(feature = "wgpu")]
	pub fn render_state(&self) -> Option<&egui_wgpu::RenderState> {
		self.render_state.as_ref()
	}

	/// Returns the `wgpu` device used to paint the frame.
	///
	/// Returns None if the `wgpu` feature is not enabled or if the frame was created without a `wgpu` context.
	#[cfg(feature = "wgpu")]
	pub fn device(&self) -> Option<&wgpu::Device> {
		self.render_state.as_ref().map(|x| &x.device)
	}

	/// Returns the `wgpu` queue used to paint the frame.
	///
	/// Returns None if the `wgpu` feature is not enabled or if the frame was created without a `wgpu` context.
	#[cfg(feature = "wgpu")]
	pub fn queue(&self) -> Option<&wgpu::Queue> {
		self.render_state.as_ref().map(|x| &x.queue)
	}

	/// Returns the native `winit` window associated with the frame.
	pub fn winit_window(&self) -> &Arc<winit::window::Window> {
		&self.window
	}

	/// Returns an owned handle for controlling the application window.
	pub fn handle(&self) -> Handle {
		self.handle.clone()
	}
}

impl std::ops::Deref for Frame {
	type Target = Handle;

	fn deref(&self) -> &Self::Target {
		&self.handle
	}
}

impl std::ops::DerefMut for Frame {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.handle
	}
}

/// A cloneable, thread-safe handle to the application window.
///
/// Before an application starts, a default handle records visibility changes
/// but has no event loop to notify. Once initialized by [`crate::window::App`],
/// its methods may be called from worker tasks to wake, show, hide, or close
/// the window.
///
/// # Examples
///
/// ```
/// use egelm::prelude::Handle;
///
/// let handle = Handle::default();
/// assert!(!handle.is_visible());
/// ```
#[derive(Debug, Clone, Default)]
pub struct Handle {
	proxy: Arc<std::sync::OnceLock<EventLoopProxy<UserEvent>>>,
	visible: Arc<AtomicBool>,
}

impl Handle {
	pub(crate) fn new() -> Self {
		Self::default()
	}

	pub(crate) fn init(&self, proxy: EventLoopProxy<UserEvent>) {
		self.proxy.set(proxy).unwrap();
	}

	/// Requests that the application process pending messages and repaint.
	///
	/// This is a no-op until the application event loop has been initialized.
	pub fn request_repaint(&self) {
		if let Some(proxy) = self.proxy.get() {
			_ = proxy.send_event(UserEvent::MessageReady);
		}
	}

	/// Hides and destroys the native window while keeping the event loop alive.
	///
	/// Calling [`show`](Self::show) later creates a new native window. Before
	/// event-loop initialization, this does nothing.
	#[cfg(not(target_os = "android"))]
	pub fn hide(&self) {
		if let Some(proxy) = self.proxy.get() {
			_ = proxy.send_event(UserEvent::Hide);
		}
		// self.visible.store(false, Ordering::Relaxed);
	}

	/// Shows the application window.
	///
	/// If the window was destroyed by [`hide`](Self::hide), it is recreated.
	/// Before event-loop initialization, this does nothing.
	#[cfg(not(target_os = "android"))]
	pub fn show(&self) {
		if let Some(proxy) = self.proxy.get() {
			_ = proxy.send_event(UserEvent::Show);
		}
		// self.visible.store(true, Ordering::Relaxed);
	}

	/// Returns whether this handle currently considers the window visible.
	pub fn is_visible(&self) -> bool {
		self.visible.load(Ordering::Relaxed)
	}

	/// Requests termination of the application event loop.
	///
	/// This is a no-op until the application event loop has been initialized.
	pub fn exit(&self) {
		if let Some(proxy) = self.proxy.get() {
			_ = proxy.send_event(UserEvent::Exit);
		}
	}
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum UserEvent {
	Show,
	Hide,
	Exit,
	RequestRepaint(Duration),
	MessageReady,
}

/// Renderer-independent application lifecycle used by the native backends.
pub(crate) trait Runner {
	fn has_window(&self) -> bool;
	fn ensure_window(&mut self, event_loop: &ActiveEventLoop);
	fn destroy_window(&mut self);
	fn update(&mut self);
	fn request_redraw(&self);
	fn set_repaint_delay(&mut self, delay: Duration);
	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent);
}

impl ApplicationHandler<UserEvent> for dyn Runner {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if !self.has_window() {
			self.ensure_window(event_loop);
		}
	}

	fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
		self.destroy_window();
	}

	fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
		if matches!(cause, StartCause::ResumeTimeReached { .. }) {
			self.request_redraw();
		}
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
		match event {
			UserEvent::Show => self.ensure_window(event_loop),
			UserEvent::Hide => self.destroy_window(),
			UserEvent::Exit => event_loop.exit(),
			UserEvent::RequestRepaint(delay) => self.set_repaint_delay(delay),
			UserEvent::MessageReady => self.update(),
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
		Runner::window_event(self, event_loop, window_id, event);
	}

	fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
		self.destroy_window();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_handle_is_hidden() {
		assert!(!Handle::default().is_visible());
	}

	#[test]
	fn cloned_handles_share_visibility_state() {
		let handle = Handle::default();
		let clone = handle.clone();

		handle.visible.store(true, Ordering::Relaxed);

		assert!(clone.is_visible());
	}

	#[test]
	fn uninitialized_handle_control_methods_are_no_ops() {
		let handle = Handle::default();

		handle.request_repaint();
		handle.exit();
		#[cfg(not(target_os = "android"))]
		{
			handle.show();
			handle.hide();
		}

		assert!(!handle.is_visible());
	}

	#[test]
	fn renderer_is_copyable_and_debuggable() {
		let renderer = Renderer::Glow;
		let copy = renderer;

		assert_eq!(format!("{renderer:?}"), "Glow");
		assert_eq!(format!("{copy:?}"), "Glow");
	}
}
