//! OpenGL rendering resources and native window controls.

use super::{Runner, UserEvent};
use crate::prelude::*;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use egui_glow::glow;
use egui_winit::winit;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::raw_window_handle::HasWindowHandle as _;

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
	egui_winit: egui_winit::State,
	viewport_info: egui::ViewportInfo,
	painter: egui_glow::Painter,
	repaint_delay: Duration,
	frame: Frame,
}

pub(crate) struct GlowRunner<T: RootWidget> {
	root: Managed<T>,
	viewport_builder: egui::ViewportBuilder,
	egui_ctx: egui::Context,
	proxy: EventLoopProxy<UserEvent>,
	surfaced: Option<Surfaced>,
	error_dialog: crate::window::error_dialog::ErrorDialog,
	handle: Handle,
	error_rx: crossbeam_channel::Receiver<T::Error>,
}

impl<T: RootWidget> GlowRunner<T> {
	pub(crate) fn new(mut root: Managed<T>, error_rx: crossbeam_channel::Receiver<T::Error>, options: ViewportBuilder, proxy: EventLoopProxy<UserEvent>) -> Self {
		let egui_ctx = egui::Context::default();
		root.setup(&egui_ctx);
		Self {
			handle: root.handle.clone(),
			root,
			viewport_builder: options,
			proxy,
			surfaced: None,
			egui_ctx,
			error_dialog: crate::window::error_dialog::ErrorDialog::default(),
			error_rx,
		}
	}

	fn redraw(&mut self, event_loop: &ActiveEventLoop) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};

		let window = surfaced.gl_window.window();
		let raw_input = surfaced.egui_winit.take_egui_input(window);
		let output = self.egui_ctx.run_ui(raw_input, |ui| {
			while let Ok(e) = self.error_rx.try_recv() {
				let (summary, details) = self.root.error(&e);
				self.error_dialog.emit(summary, details);
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

		let mut capture_data = Vec::new();
		for (_, egui::ViewportOutput { commands, .. }) in output.viewport_output {
			let mut actions_requested = Default::default();
			egui_winit::process_viewport_commands(&self.egui_ctx, &mut surfaced.viewport_info, commands, window, &mut actions_requested);
			for action in actions_requested {
				match action {
					egui_winit::ActionRequested::Screenshot(data) => capture_data.push(data),
					_ => tracing::warn!(?action, "viewport action is not supported by the glow backend"),
				}
			}
		}

		let clear_color = self.root.clear_color();
		surfaced
			.egui_winit
			.handle_platform_output(window, output.platform_output);

		let primitives = self
			.egui_ctx
			.tessellate(output.shapes, output.pixels_per_point);
		let size = window.inner_size();
		let paint_size = [size.width.max(1), size.height.max(1)];

		surfaced.painter.clear(
			paint_size,
			[
				clear_color[0] as f32 / 255.0,
				clear_color[1] as f32 / 255.0,
				clear_color[2] as f32 / 255.0,
				clear_color[3] as f32 / 255.0,
			],
		);

		surfaced
			.painter
			.paint_and_update_textures(paint_size, output.pixels_per_point, &primitives, &output.textures_delta);

		if !capture_data.is_empty() {
			let image = Arc::new(surfaced.painter.read_screen_rgba(paint_size));
			for user_data in capture_data {
				surfaced
					.egui_winit
					.egui_input_mut()
					.events
					.push(egui::Event::Screenshot {
						viewport_id: egui::ViewportId::ROOT,
						user_data,
						image: Arc::clone(&image),
					});
			}
		}
		surfaced
			.gl_window
			.swap_buffers()
			.expect("could not swap buffers");

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

impl<T: RootWidget> Runner for GlowRunner<T> {
	fn has_window(&self) -> bool {
		self.surfaced.is_some()
	}

	fn shutdown_root(&mut self) {
		self.root.shutdown();
	}

	#[cfg(feature = "accesskit")]
	fn access_kit_event(&mut self, event: egui_winit::accesskit_winit::Event) {
		let Some(surfaced) = &mut self.surfaced else {
			return;
		};

		if event.window_id != surfaced.gl_window.window().id() {
			return;
		}

		match event.window_event {
			egui_winit::accesskit_winit::WindowEvent::ActionRequested(request) => {
				surfaced.egui_winit.on_accesskit_action_request(request);
				surfaced.gl_window.window().request_redraw();
			}
			egui_winit::accesskit_winit::WindowEvent::InitialTreeRequested => {
				surfaced.gl_window.window().request_redraw();
			}
			egui_winit::accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
		}
	}

	#[tracing::instrument(skip(self, event_loop))]
	fn ensure_window(&mut self, event_loop: &ActiveEventLoop) {
		if self.surfaced.is_some() {
			tracing::trace!("glow window already exists");
			return;
		}
		tracing::info!("creating glow window and rendering context");

		let gl_window = unsafe { GlutinWindowContext::new(&self.egui_ctx, event_loop, &self.viewport_builder) };

		let gl = Arc::new(unsafe {
			glow::Context::from_loader_function(|s| {
				let s = CString::new(s).expect("proc name should not contain nul bytes");
				gl_window.get_proc_address(&s)
			})
		});

		let painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, true).expect("failed to create the egui Glow painter");

		let mut egui_winit = egui_winit::State::new(
			self.egui_ctx.clone(),
			egui::ViewportId::ROOT,
			event_loop,
			None,
			event_loop.system_theme(),
			Some(painter.max_texture_side()),
		);
		egui_winit.set_max_texture_side(painter.max_texture_side());

		#[cfg(feature = "accesskit")]
		egui_winit.init_accesskit(event_loop, gl_window.window(), self.proxy.clone());

		let proxy = self.proxy.clone();
		self.egui_ctx.set_request_repaint_callback(move |info| {
			_ = proxy.send_event(UserEvent::RequestRepaint(info.delay));
		});

		gl_window.window().set_visible(true);

		self.surfaced = Some(Surfaced {
			frame: Frame {
				handle: self.handle.clone(),
				gl: Some(gl),
				window: gl_window.window.clone(),
				#[cfg(feature = "wgpu")]
				render_state: None,
			},
			egui_winit,
			viewport_info: egui::ViewportInfo::default(),
			painter,
			gl_window,
			repaint_delay: Duration::MAX,
		});
		self.handle.visible.store(true, Ordering::Relaxed);
		tracing::info!(window_id = ?self.surfaced.as_ref().map(|surface| surface.gl_window.window().id()), "glow window ready");
	}

	fn destroy_window(&mut self) {
		if let Some(mut surfaced) = self.surfaced.take() {
			tracing::info!(window_id = ?surfaced.gl_window.window().id(), "destroying glow window");
			surfaced.painter.destroy();
		}
		self.handle.visible.store(false, Ordering::Relaxed);
	}

	fn update(&mut self) {
		let span = tracing::span!(tracing::Level::TRACE, "app_tick", backend = "glow");
		let _enter = span.enter();
		if let Err(e) = self.root.update() {
			tracing::error!(error = ?e, "root widget update failed");
			let (summary, details) = self.root.error(&e);
			self.error_dialog.emit(summary, details);
		}
		if let Some(surfaced) = self.surfaced.as_mut() {
			surfaced.gl_window.window().request_redraw();
		}
	}

	fn request_redraw(&self) {
		if let Some(surfaced) = &self.surfaced {
			surfaced.gl_window.window().request_redraw();
		}
	}

	fn set_repaint_delay(&mut self, delay: Duration) {
		if let Some(surfaced) = self.surfaced.as_mut() {
			surfaced.repaint_delay = delay;
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};

		if surfaced.gl_window.window().id() != window_id {
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

		if let WindowEvent::Resized(physical_size) = &event {
			surfaced.gl_window.resize(*physical_size);
		}

		let response = surfaced
			.egui_winit
			.on_window_event(surfaced.gl_window.window(), &event);
		if response.repaint {
			surfaced.gl_window.window().request_redraw();
		}
	}
}
