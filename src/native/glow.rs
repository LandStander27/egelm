//! OpenGL rendering resources and native window controls.

use crate::prelude::*;
use crate::widgets::prelude::*;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use egui_glow::glow;
use egui_winit::winit;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::raw_window_handle::HasWindowHandle as _;

/// Rendering resources and window controls for the current frame.
///
/// A mutable `Frame` is passed to widget rendering methods. It dereferences to
/// [`Handle`], allowing window-control methods such as [`Handle::exit`] to be
/// called directly on it.
pub struct Frame {
	handle: Handle,
	gl: Arc<glow::Context>,
	window: Arc<winit::window::Window>,
}

impl Frame {
	/// Returns the OpenGL context used to paint the frame.
	pub fn gl(&self) -> &Arc<glow::Context> {
		&self.gl
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
	/// event-loop initialization, this only records the handle as hidden.
	pub fn hide(&self) {
		if let Some(proxy) = self.proxy.get() {
			_ = proxy.send_event(UserEvent::Hide);
		}
		// self.visible.store(false, Ordering::Relaxed);
	}

	/// Shows the application window.
	///
	/// If the window was destroyed by [`hide`](Self::hide), it is recreated.
	/// Before event-loop initialization, this only records the handle as visible.
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum UserEvent {
	Show,
	Hide,
	Exit,
	RequestRepaint(Duration),
	MessageReady,
}

struct GlutinWindowContext {
	window: Arc<winit::window::Window>,
	gl_context: glutin::context::PossiblyCurrentContext,
	gl_display: glutin::display::Display,
	gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlutinWindowContext {
	unsafe fn new(egui_ctx: &egui::Context, event_loop: &ActiveEventLoop, options: &ViewportBuilder) -> Self {
		use glutin::context::NotCurrentGlContext as _;
		use glutin::display::GetGlDisplay as _;
		use glutin::display::GlDisplay as _;
		use glutin::prelude::GlSurface as _;

		let window_attributes = egui_winit::create_winit_window_attributes(egui_ctx, options.clone());
		let config_template_builder = glutin::config::ConfigTemplateBuilder::new()
			.prefer_hardware_accelerated(None)
			.with_depth_size(0)
			.with_stencil_size(0)
			.with_transparency(options.transparent.unwrap_or(false));

		let (mut window, gl_config) = glutin_winit::DisplayBuilder::new()
			.with_preference(glutin_winit::ApiPreference::FallbackEgl)
			.with_window_attributes(Some(window_attributes.clone()))
			.build(event_loop, config_template_builder, |mut config_iterator| {
				config_iterator
					.next()
					.expect("could not find a matching configuration for creating glutin config")
			})
			.expect("could not create gl_config");

		if let Some(window) = &window {
			egui_winit::apply_viewport_builder_to_window(egui_ctx, window, options);
		}

		let gl_display = gl_config.display();

		let raw_window_handle = window.as_ref().map(|x| {
			x.window_handle()
				.expect("could not get window handle")
				.as_raw()
		});

		let context_attributes = glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
		let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
			.with_context_api(glutin::context::ContextApi::Gles(None))
			.build(raw_window_handle);
		let not_current_gl_context = unsafe {
			gl_display
				.create_context(&gl_config, &context_attributes)
				.unwrap_or_else(|_| {
					gl_config
						.display()
						.create_context(&gl_config, &fallback_context_attributes)
						.expect("could not create context even with fallback attributes")
				})
		};

		let window = window
			.take()
			.unwrap_or_else(|| glutin_winit::finalize_window(event_loop, window_attributes.clone(), &gl_config).expect("could not finalize glutin window"));
		let (width, height): (u32, u32) = window.inner_size().into();
		let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
		let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);
		let surface_attributes = glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new().build(
			window
				.window_handle()
				.expect("could not get window handle")
				.as_raw(),
			width,
			height,
		);
		let gl_surface = unsafe {
			gl_display
				.create_window_surface(&gl_config, &surface_attributes)
				.unwrap()
		};
		let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

		gl_surface
			.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::MIN))
			.unwrap();

		Self {
			window: Arc::new(window),
			gl_context,
			gl_display,
			gl_surface,
		}
	}

	fn window(&self) -> &winit::window::Window {
		&self.window
	}

	fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
		use glutin::surface::GlSurface as _;
		self.gl_surface.resize(
			&self.gl_context,
			physical_size.width.try_into().unwrap_or(NonZeroU32::MIN),
			physical_size.height.try_into().unwrap_or(NonZeroU32::MIN),
		);
	}

	fn swap_buffers(&self) -> glutin::error::Result<()> {
		use glutin::surface::GlSurface as _;
		self.gl_surface.swap_buffers(&self.gl_context)
	}

	fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
		use glutin::display::GlDisplay as _;
		self.gl_display.get_proc_address(addr)
	}
}

struct Surfaced {
	gl_window: GlutinWindowContext,
	egui_glow: egui_glow::EguiGlow,
	repaint_delay: Duration,
	frame: Frame,
}

pub(crate) struct Runner<T: RootWidget> {
	root: Managed<T>,
	viewport_builder: egui::ViewportBuilder,
	egui_ctx: egui::Context,
	proxy: EventLoopProxy<UserEvent>,
	surfaced: Option<Surfaced>,
	error_dialog: crate::window::error_dialog::ErrorDialog,
	handle: Handle,
}

impl<T: RootWidget> Runner<T> {
	pub(crate) fn new(root: Managed<T>, options: ViewportBuilder, proxy: EventLoopProxy<UserEvent>) -> Self {
		Self {
			handle: root.handle.clone(),
			root,
			viewport_builder: options,
			proxy,
			surfaced: None,
			egui_ctx: egui::Context::default(),
			error_dialog: crate::window::error_dialog::ErrorDialog::default(),
		}
	}

	fn update(&mut self) {
		let span = tracing::span!(tracing::Level::INFO, "app_tick");
		let _enter = span.enter();
		if let Err(e) = self.root.update() {
			let (summary, details) = self.root.error(&e);
			self.error_dialog.emit(summary, details);
		}
		if let Some(surfaced) = self.surfaced.as_mut() {
			surfaced.gl_window.window().request_redraw();
		}
	}

	fn ensure_window(&mut self, event_loop: &ActiveEventLoop) {
		if self.surfaced.is_some() {
			return;
		}

		let gl_window = unsafe { GlutinWindowContext::new(&self.egui_ctx, event_loop, &self.viewport_builder) };

		let gl = Arc::new(unsafe {
			glow::Context::from_loader_function(|s| {
				let s = CString::new(s).expect("proc name should not contain nul bytes");
				gl_window.get_proc_address(&s)
			})
		});

		let egui_glow = egui_glow::EguiGlow::new(event_loop, Arc::clone(&gl), None, None, true);
		self.root.setup(&egui_glow.egui_ctx);

		let proxy = self.proxy.clone();
		egui_glow
			.egui_ctx
			.set_request_repaint_callback(move |info| {
				_ = proxy.send_event(UserEvent::RequestRepaint(info.delay));
			});

		gl_window.window().set_visible(true);

		self.surfaced = Some(Surfaced {
			frame: Frame {
				handle: self.handle.clone(),
				gl,
				window: gl_window.window.clone(),
			},
			egui_glow,
			gl_window,
			repaint_delay: Duration::MAX,
		});
		self.handle.visible.store(true, Ordering::Relaxed);
	}

	fn destroy_window(&mut self) {
		if let Some(mut surfaced) = self.surfaced.take() {
			surfaced.egui_glow.destroy();
		}
		self.handle.visible.store(true, Ordering::Relaxed);
	}

	fn redraw(&mut self, event_loop: &ActiveEventLoop) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};

		let window = surfaced.gl_window.window();
		surfaced.egui_glow.run(window, |ui| {
			let rx = crate::window::ERROR_RX
				.get()
				.expect("ERROR_TX not inited; was App::new called?");
			let lock = rx.lock().expect("app crashed");
			while let Ok(e) = lock.try_recv() {
				match e.as_any().downcast_ref::<T::Error>() {
					Some(err) => {
						let (summary, details) = self.root.error(err);
						self.error_dialog.emit(summary, details);
					}
					None => tracing::error!("{}", Error::UnknownErrorFromRoot(e)),
				}
			}

			self.root.render(ui, &mut surfaced.frame);

			#[cfg(debug_assertions)]
			ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
				ui.add_space(12.0);
				ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
					ui.add_space(12.0);
					ui.label(
						RichText::new("⚠ Debug build ⚠")
							.small()
							.color(ui.visuals().warn_fg_color),
					)
					.on_hover_text("This is a debug build. Expect bugs.");
				})
			});

			self.error_dialog.render(ui, &mut surfaced.frame);
		});

		unsafe {
			use glow::HasContext as _;
			surfaced
				.frame
				.gl
				.clear_color(27.0 / 255.0, 27.0 / 255.0, 27.0 / 255.0, 1.0);
			surfaced.frame.gl.clear(glow::COLOR_BUFFER_BIT);
		}
		surfaced.egui_glow.paint(surfaced.gl_window.window());
		surfaced
			.gl_window
			.swap_buffers()
			.expect("could not swap buffers");
		surfaced.gl_window.window().set_visible(true);

		event_loop.set_control_flow(if surfaced.repaint_delay.is_zero() {
			surfaced.gl_window.window().request_redraw();
			winit::event_loop::ControlFlow::Poll
		} else if let Some(repaint_after_instant) = std::time::Instant::now().checked_add(surfaced.repaint_delay) {
			winit::event_loop::ControlFlow::WaitUntil(repaint_after_instant)
		} else {
			winit::event_loop::ControlFlow::Wait
		});
	}
}

impl<T: RootWidget> ApplicationHandler<UserEvent> for Runner<T> {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.viewport_builder.visible.unwrap_or(true) && self.surfaced.is_none() {
			self.ensure_window(event_loop);
		}
	}

	fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
		if let StartCause::ResumeTimeReached { .. } = cause
			&& let Some(surfaced) = &self.surfaced
		{
			surfaced.gl_window.window().request_redraw();
		}
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
		match event {
			UserEvent::Show => self.ensure_window(event_loop),
			UserEvent::Hide => self.destroy_window(),
			UserEvent::Exit => event_loop.exit(),
			UserEvent::RequestRepaint(delay) => {
				if let Some(surfaced) = self.surfaced.as_mut() {
					surfaced.repaint_delay = delay;
				}
			}
			UserEvent::MessageReady => {
				self.update();
			}
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};

		if surfaced.gl_window.window().id() != window_id {
			return;
		}

		if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
			self.root.close(&mut surfaced.frame);
			return;
		}

		if matches!(event, WindowEvent::RedrawRequested) {
			self.redraw(event_loop);
			return;
		}

		if let WindowEvent::Resized(physical_size) = &event {
			surfaced.gl_window.resize(*physical_size);
		}

		let response = surfaced
			.egui_glow
			.on_window_event(surfaced.gl_window.window(), &event);
		if response.repaint {
			surfaced.gl_window.window().request_redraw();
		}
	}

	fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
		self.destroy_window();
	}
}
