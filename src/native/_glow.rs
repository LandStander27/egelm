//! OpenGL rendering resources.

use super::{Backend, Handle, RenderFrame};
use crate::prelude::*;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::sync::Arc;

use egui_glow::glow;
use egui_winit::winit;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::HasWindowHandle as _;

#[derive(Default)]
pub(crate) struct GlowBackend {
	gl_state: Option<GlowState>,
	surface: Option<GlowSurface>,
}

struct GlowState {
	gl_display: glutin::display::Display,
	gl_config: glutin::config::Config,
	gl_context: Option<glutin::context::PossiblyCurrentContext>,
	gl: Arc<glow::Context>,
	painter: egui_glow::Painter,
}

struct GlowSurface {
	window: Arc<winit::window::Window>,
	gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlowBackend {
	fn create_or_recreate(&mut self, egui_ctx: &egui::Context, event_loop: &ActiveEventLoop, options: &ViewportBuilder) {
		use glutin::context::NotCurrentGlContext as _;
		use glutin::context::PossiblyCurrentGlContext as _;
		use glutin::display::{GetGlDisplay as _, GlDisplay as _};
		use glutin::prelude::GlSurface as _;

		let window_attributes = egui_winit::create_winit_window_attributes(egui_ctx, options.clone());

		if let Some(state) = &mut self.gl_state {
			// Recreate surface and window using existing state.
			let window = glutin_winit::finalize_window(event_loop, window_attributes, &state.gl_config).expect("could not finalize glutin window");
			egui_winit::apply_viewport_builder_to_window(egui_ctx, &window, options);

			let size = window.inner_size();
			let surface_attributes = glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new().build(
				window
					.window_handle()
					.expect("could not get window handle")
					.as_raw(),
				NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN),
				NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN),
			);
			let gl_surface = unsafe {
				state
					.gl_display
					.create_window_surface(&state.gl_config, &surface_attributes)
					.expect("could not create OpenGL window surface")
			};

			let ctx = state.gl_context.take().expect("gl_context was missing");
			let not_current = ctx
				.make_not_current()
				.expect("could not make context not current");
			let gl_context = not_current
				.make_current(&gl_surface)
				.expect("could not make OpenGL context current on new surface");

			gl_surface
				.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::MIN))
				.unwrap_or_else(|e| tracing::warn!(error = %e, "could not enable vertical synchronization"));

			state.gl_context = Some(gl_context);

			self.surface = Some(GlowSurface {
				window: Arc::new(window),
				gl_surface,
			});
			return;
		}

		// First time initialization
		let config_template = glutin::config::ConfigTemplateBuilder::new()
			.prefer_hardware_accelerated(None)
			.with_depth_size(0)
			.with_stencil_size(0)
			.with_transparency(options.transparent.unwrap_or(false));
		let (mut window, gl_config) = glutin_winit::DisplayBuilder::new()
			.with_preference(glutin_winit::ApiPreference::FallbackEgl)
			.with_window_attributes(Some(window_attributes.clone()))
			.build(event_loop, config_template, |mut configs| {
				configs
					.next()
					.expect("could not find a matching glutin configuration")
			})
			.expect("could not create glutin configuration");

		if let Some(window) = &window {
			egui_winit::apply_viewport_builder_to_window(egui_ctx, window, options);
		}
		let gl_display = gl_config.display();
		let raw_window_handle = window.as_ref().map(|window| {
			window
				.window_handle()
				.expect("could not get window handle")
				.as_raw()
		});
		let context_attributes = glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
		let fallback_attributes = glutin::context::ContextAttributesBuilder::new()
			.with_context_api(glutin::context::ContextApi::Gles(None))
			.build(raw_window_handle);
		let not_current = unsafe {
			gl_display
				.create_context(&gl_config, &context_attributes)
				.unwrap_or_else(|_| {
					gl_display
						.create_context(&gl_config, &fallback_attributes)
						.expect("could not create OpenGL or OpenGL ES context")
				})
		};
		let window = window
			.take()
			.unwrap_or_else(|| glutin_winit::finalize_window(event_loop, window_attributes, &gl_config).expect("could not finalize glutin window"));
		let size = window.inner_size();
		let surface_attributes = glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new().build(
			window
				.window_handle()
				.expect("could not get window handle")
				.as_raw(),
			NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN),
			NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN),
		);
		let gl_surface = unsafe {
			gl_display
				.create_window_surface(&gl_config, &surface_attributes)
				.expect("could not create OpenGL window surface")
		};
		let gl_context = not_current
			.make_current(&gl_surface)
			.expect("could not make OpenGL context current");
		gl_surface
			.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::MIN))
			.unwrap_or_else(|e| tracing::warn!(error = %e, "could not enable vertical synchronization"));

		let gl = Arc::new(unsafe {
			glow::Context::from_loader_function(|name| {
				let name = CString::new(name).expect("OpenGL procedure name contained a nul byte");
				gl_display.get_proc_address(&name)
			})
		});
		let painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, true).expect("could not create egui Glow painter");

		self.gl_state = Some(GlowState {
			gl_display,
			gl_config,
			gl_context: Some(gl_context),
			gl,
			painter,
		});

		self.surface = Some(GlowSurface {
			window: Arc::new(window),
			gl_surface,
		});
	}

	fn surface(&self) -> &GlowSurface {
		self.surface
			.as_ref()
			.expect("glow surface was not initialized")
	}
}

impl Backend for GlowBackend {
	fn name(&self) -> &'static str {
		"glow"
	}

	fn create_surface(&mut self, egui_ctx: &egui::Context, event_loop: &ActiveEventLoop, viewport_builder: &ViewportBuilder) {
		self.create_or_recreate(egui_ctx, event_loop, viewport_builder);
	}

	fn window(&self) -> &Arc<winit::window::Window> {
		&self.surface().window
	}

	fn max_texture_side(&self) -> usize {
		self.gl_state.as_ref().unwrap().painter.max_texture_side()
	}

	fn frame(&self, handle: Handle) -> Frame {
		let state = self.gl_state.as_ref().unwrap();
		let surface = self.surface();
		Frame {
			handle,
			gl: Some(Arc::clone(&state.gl)),
			window: surface.window.clone(),
			#[cfg(feature = "wgpu")]
			render_state: None,
		}
	}

	fn paint(&mut self, frame: RenderFrame<'_>) {
		use glutin::surface::GlSurface as _;

		let state = self.gl_state.as_mut().unwrap();
		let surface = self.surface.as_mut().unwrap();
		let size = surface.window.inner_size();
		let paint_size = [size.width.max(1), size.height.max(1)];
		state.painter.clear(paint_size, frame.clear_color);
		state
			.painter
			.paint_and_update_textures(paint_size, frame.pixels_per_point, frame.primitives, frame.textures_delta);
		if !frame.screenshots.is_empty() {
			let image = Arc::new(state.painter.read_screen_rgba(paint_size));
			for user_data in frame.screenshots {
				frame.events.push(egui::Event::Screenshot {
					viewport_id: egui::ViewportId::ROOT,
					user_data,
					image: Arc::clone(&image),
				});
			}
		}
		surface
			.gl_surface
			.swap_buffers(state.gl_context.as_ref().unwrap())
			.expect("could not swap OpenGL buffers");
	}

	fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
		use glutin::surface::GlSurface as _;
		let state = self.gl_state.as_ref().unwrap();
		let surface = self.surface.as_mut().unwrap();
		surface.gl_surface.resize(
			state.gl_context.as_ref().unwrap(),
			NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN),
			NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN),
		);
	}

	fn destroy_surface(&mut self) {
		if self.surface.take().is_some()
			&& let Some(state) = self.gl_state.as_mut()
		{
			use glutin::context::NotCurrentGlContext as _;
			use glutin::context::PossiblyCurrentGlContext as _;
			if let Some(ctx) = state.gl_context.take() {
				state.gl_context = Some(
					ctx.make_not_current()
						.expect("Failed to make gl context not current")
						.treat_as_possibly_current(),
				);
			}
		}
		{}
	}
}
