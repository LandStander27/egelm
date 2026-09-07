//! Native platform integrations used by `egelm`.

use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

#[cfg(glow)]
use egui_glow::glow;

#[cfg(wgpu)]
use egui_wgpu::wgpu;
use egui_winit::winit::{
	self,
	application::ApplicationHandler,
	event::{StartCause, WindowEvent},
	event_loop::{ActiveEventLoop, EventLoopProxy},
};

use crate::window::LeafWidget;

/// OpenGL backend implementation.
#[cfg(glow)]
pub(crate) mod _glow;

/// `wgpu` backend implementation.
#[cfg(wgpu)]
pub(crate) mod _wgpu;

/// Renderer selection for the app.
///
/// The `Glow` variant uses OpenGL via the `glow` crate, while `Wgpu` uses the
/// `wgpu` crate. When both features are enabled, [`crate::window::App::run`]
/// prefers `Glow`; use [`crate::window::App::run_with_backend`] to select one
/// explicitly.
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

	#[cfg(glow)]
	gl: Option<Arc<glow::Context>>,

	#[cfg(wgpu)]
	render_state: Option<egui_wgpu::RenderState>,

	window: Arc<winit::window::Window>,
}

impl Frame {
	/// Returns the OpenGL context used to paint the frame.
	///
	/// Returns None if the `glow` feature is not enabled or if the frame was created without an OpenGL context.
	#[cfg(glow)]
	pub fn gl(&self) -> Option<&Arc<glow::Context>> {
		self.gl.as_ref()
	}

	/// Returns the `wgpu` state used to paint the frame.
	///
	/// Returns None if the `wgpu` feature is not enabled or if the frame was created without a `wgpu` context.
	#[cfg(wgpu)]
	pub fn render_state(&self) -> Option<&egui_wgpu::RenderState> {
		self.render_state.as_ref()
	}

	/// Returns the `wgpu` device used to paint the frame.
	///
	/// Returns None if the `wgpu` feature is not enabled or if the frame was created without a `wgpu` context.
	#[cfg(wgpu)]
	pub fn device(&self) -> Option<&wgpu::Device> {
		self.render_state.as_ref().map(|x| &x.device)
	}

	/// Returns the `wgpu` queue used to paint the frame.
	///
	/// Returns None if the `wgpu` feature is not enabled or if the frame was created without a `wgpu` context.
	#[cfg(wgpu)]
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
		tracing::debug!("application handle initialized");
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
	#[cfg(not(android))]
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
	#[cfg(not(android))]
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
#[derive(Debug)]
pub(crate) enum UserEvent {
	Show,
	Hide,
	Exit,
	RequestRepaint(Duration),
	MessageReady,

	#[cfg(feature = "accesskit")]
	AccessKit(egui_winit::accesskit_winit::Event),
}

#[cfg(feature = "accesskit")]
impl From<egui_winit::accesskit_winit::Event> for UserEvent {
	fn from(value: egui_winit::accesskit_winit::Event) -> Self {
		Self::AccessKit(value)
	}
}

/// Renderer-specific resources used by the shared [`Runner`].
///
/// A backend owns its surface and implements only graphics-specific behavior.
/// Window lifecycle, egui integration, application updates, viewport commands,
/// accessibility, and repaint scheduling are handled by `Runner`.
pub(crate) trait Backend: 'static {
	/// Returns the backend name used in diagnostics.
	fn name(&self) -> &'static str;

	/// Creates and stores the backend's window surface.
	fn create_surface(&mut self, egui_ctx: &egui::Context, event_loop: &ActiveEventLoop, viewport_builder: &egui::ViewportBuilder);

	/// Returns the window for the currently stored surface.
	fn window(&self) -> &Arc<winit::window::Window>;

	/// Returns the largest texture dimension supported by the backend.
	fn max_texture_side(&self) -> usize;

	/// Exposes the current backend resources to widget rendering.
	fn frame(&self, handle: Handle) -> Frame;

	/// Delivers backend results that must be added to the next egui input.
	fn prepare_frame(&mut self, _events: &mut Vec<egui::Event>) {}

	/// Paints one renderer-ready egui frame.
	fn paint(&mut self, frame: RenderFrame<'_>);

	/// Resizes the stored surface when required by the backend.
	fn resize(&mut self, _size: winit::dpi::PhysicalSize<u32>) {}

	/// Releases the stored surface while preserving reusable backend state.
	fn destroy_surface(&mut self) {}
}

/// Renderer-ready output produced by the shared egui runner.
pub(crate) struct RenderFrame<'a> {
	pub(crate) pixels_per_point: f32,
	pub(crate) clear_color: [f32; 4],
	pub(crate) primitives: &'a [egui::ClippedPrimitive],
	pub(crate) textures_delta: &'a mut egui::TexturesDelta,
	pub(crate) screenshots: Vec<egui::UserData>,
	#[allow(dead_code)] // Used by synchronous screenshot backends such as Glow.
	pub(crate) events: &'a mut Vec<egui::Event>,
}

struct Surfaced {
	egui_winit: egui_winit::State,
	viewport_info: egui::ViewportInfo,
	repaint_delay: Duration,
	frame: Frame,
}

/// Shared native application runner using a dynamically selected backend.
pub(crate) struct Runner<T: crate::window::RootWidget> {
	root: crate::window::Managed<T>,
	viewport_builder: egui::ViewportBuilder,
	egui_ctx: egui::Context,
	proxy: EventLoopProxy<UserEvent>,
	backend: Box<dyn Backend>,
	surfaced: Option<Surfaced>,
	error_dialog: crate::window::error_dialog::ErrorDialog,
	handle: Handle,
	error_rx: crossbeam_channel::Receiver<T::Error>,

	_themer: Option<crate::theme::ThemeWatcher>,
}

impl<T: crate::window::RootWidget> Runner<T> {
	pub(crate) fn new(
		mut root: crate::window::Managed<T>,
		error_rx: crossbeam_channel::Receiver<T::Error>,
		options: egui::ViewportBuilder,
		proxy: EventLoopProxy<UserEvent>,
		backend: Box<dyn Backend>,
	) -> Self {
		let egui_ctx = egui::Context::default();
		root.setup(&egui_ctx);
		Self {
			handle: root.handle.clone(),
			root,
			viewport_builder: options,
			proxy,
			backend,
			surfaced: None,

			#[cfg(feature = "theming")]
			_themer: crate::theme::ThemeWatcher::new(&egui_ctx),

			egui_ctx,
			error_dialog: crate::window::error_dialog::ErrorDialog::default(),
			error_rx,
		}
	}

	fn redraw(&mut self, event_loop: &ActiveEventLoop) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};
		let window = self.backend.window().clone();

		self.backend
			.prepare_frame(&mut surfaced.egui_winit.egui_input_mut().events);
		let raw_input = surfaced.egui_winit.take_egui_input(&window);

		let mut output = self.egui_ctx.run_ui(raw_input, |ui| {
			while let Ok(error) = self.error_rx.try_recv() {
				let (summary, details) = self.root.error(&error);
				self.error_dialog.emit(summary, details);
			}

			self.root.render(ui, &mut surfaced.frame);

			#[cfg(debug_assertions)]
			ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
				ui.add_space(12.0);
				ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
					ui.add_space(12.0);
					ui.label(
						egui::RichText::new("⚠ Debug build ⚠")
							.small()
							.color(ui.visuals().warn_fg_color),
					)
					.on_hover_text("This is a debug build. Expect bugs.");
				})
			});

			self.error_dialog.render(ui, &mut surfaced.frame);
		});

		let mut screenshots = Vec::new();
		for (_, egui::ViewportOutput { commands, .. }) in output.viewport_output {
			let mut actions_requested = Vec::new();
			egui_winit::process_viewport_commands(&self.egui_ctx, &mut surfaced.viewport_info, commands, &window, &mut actions_requested);
			for action in actions_requested {
				match action {
					egui_winit::ActionRequested::Screenshot(data) => screenshots.push(data),
					_ => tracing::warn!(?action, backend = self.backend.name(), "viewport action is not supported"),
				}
			}
		}
		surfaced
			.egui_winit
			.handle_platform_output(&window, output.platform_output);
		let primitives = self
			.egui_ctx
			.tessellate(output.shapes, output.pixels_per_point);
		self.backend.paint(RenderFrame {
			pixels_per_point: output.pixels_per_point,
			clear_color: {
				let bg = self.egui_ctx.global_style().visuals.window_fill;
				[bg.r() as f32 / 255.0, bg.g() as f32 / 255.0, bg.b() as f32 / 255.0, bg.a() as f32 / 255.0]
			},
			primitives: &primitives,
			textures_delta: &mut output.textures_delta,
			screenshots,
			events: &mut surfaced.egui_winit.egui_input_mut().events,
		});
		window.set_visible(true);

		event_loop.set_control_flow(if surfaced.repaint_delay.is_zero() {
			window.request_redraw();
			winit::event_loop::ControlFlow::Poll
		} else if let Some(instant) = std::time::Instant::now().checked_add(surfaced.repaint_delay) {
			winit::event_loop::ControlFlow::WaitUntil(instant)
		} else {
			winit::event_loop::ControlFlow::Wait
		});
	}

	fn has_window(&self) -> bool {
		self.surfaced.is_some()
	}

	fn ensure_window(&mut self, event_loop: &ActiveEventLoop) {
		if self.surfaced.is_some() {
			tracing::trace!(backend = self.backend.name(), "window already exists");
			return;
		}
		tracing::info!(backend = self.backend.name(), "creating window and rendering surface");
		self.backend
			.create_surface(&self.egui_ctx, event_loop, &self.viewport_builder);
		let window = self.backend.window().clone();
		#[allow(unused_mut)]
		let mut egui_winit = egui_winit::State::new(
			self.egui_ctx.clone(),
			egui::ViewportId::ROOT,
			event_loop,
			None,
			event_loop.system_theme(),
			Some(self.backend.max_texture_side()),
		);

		#[cfg(feature = "accesskit")]
		egui_winit.init_accesskit(event_loop, &window, self.proxy.clone());

		let proxy = self.proxy.clone();
		self.egui_ctx.set_request_repaint_callback(move |info| {
			_ = proxy.send_event(UserEvent::RequestRepaint(info.delay));
		});

		let frame = self.backend.frame(self.handle.clone());
		let window_id = window.id();
		window.set_visible(true);
		self.surfaced = Some(Surfaced {
			egui_winit,
			viewport_info: egui::ViewportInfo::default(),
			repaint_delay: Duration::MAX,
			frame,
		});
		self.handle.visible.store(true, Ordering::Relaxed);
		tracing::info!(?window_id, backend = self.backend.name(), "window ready");
	}

	fn destroy_window(&mut self) {
		if self.surfaced.take().is_some() {
			tracing::info!(window_id = ?self.backend.window().id(), backend = self.backend.name(), "destroying window");
			self.backend.destroy_surface();
		}
		self.handle.visible.store(false, Ordering::Relaxed);
	}

	fn update(&mut self) {
		let span = tracing::span!(tracing::Level::TRACE, "app_tick", backend = self.backend.name());
		let _enter = span.enter();
		if let Err(error) = self.root.update() {
			tracing::error!(?error, "root widget update failed");
			let (summary, details) = self.root.error(&error);
			self.error_dialog.emit(summary, details);
		}
		self.request_redraw();
	}

	fn request_redraw(&self) {
		if self.surfaced.is_some() {
			self.backend.window().request_redraw();
		}
	}

	fn set_repaint_delay(&mut self, delay: Duration) {
		if let Some(surfaced) = &mut self.surfaced {
			surfaced.repaint_delay = delay;
		}
	}

	#[cfg(feature = "accesskit")]
	fn access_kit_event(&mut self, event: egui_winit::accesskit_winit::Event) {
		let Some(surfaced) = &mut self.surfaced else {
			return;
		};
		let window = self.backend.window();
		if event.window_id != window.id() {
			return;
		}
		match event.window_event {
			egui_winit::accesskit_winit::WindowEvent::ActionRequested(request) => {
				surfaced.egui_winit.on_accesskit_action_request(request);
				window.request_redraw();
			}
			egui_winit::accesskit_winit::WindowEvent::InitialTreeRequested => window.request_redraw(),
			egui_winit::accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
		}
	}
}

impl<T: crate::window::RootWidget> ApplicationHandler<UserEvent> for Runner<T> {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		tracing::info!("application resumed");
		if !self.has_window() {
			self.ensure_window(event_loop);
		}
	}

	fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
		tracing::info!("application suspended");
		self.destroy_window();
	}

	fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
		if matches!(cause, StartCause::ResumeTimeReached { .. }) {
			self.request_redraw();
		}
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
		tracing::trace!(?event, "processing user event");
		match event {
			UserEvent::Show => self.ensure_window(event_loop),
			UserEvent::Hide => self.destroy_window(),
			UserEvent::Exit => event_loop.exit(),
			UserEvent::RequestRepaint(delay) => self.set_repaint_delay(delay),
			UserEvent::MessageReady => self.update(),

			#[cfg(feature = "accesskit")]
			UserEvent::AccessKit(event) => self.access_kit_event(event),
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};
		let window = self.backend.window().clone();
		if window.id() != window_id {
			return;
		}
		if matches!(event, WindowEvent::CloseRequested) {
			self.root.close(&mut surfaced.frame);
			return;
		}
		if matches!(event, WindowEvent::Destroyed) {
			self.destroy_window();
			return;
		}
		if matches!(event, WindowEvent::RedrawRequested) {
			self.redraw(event_loop);
			return;
		}
		if let WindowEvent::Resized(size) = event {
			self.backend.resize(size);
		}
		let response = surfaced.egui_winit.on_window_event(&window, &event);
		if response.repaint {
			window.request_redraw();
		}
	}

	fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
		tracing::info!("application event loop exiting");
		self.root.shutdown();
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
