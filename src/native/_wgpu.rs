//! `wgpu` rendering resources and native window controls.

use super::{Runner, UserEvent};
use crate::prelude::*;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use egui_winit::winit;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

struct Surfaced {
	window: Arc<winit::window::Window>,
	egui_winit: egui_winit::State,
	viewport_info: egui::ViewportInfo,
	repaint_delay: Duration,
	frame: Frame,
}

pub(crate) struct WgpuRunner<T: RootWidget> {
	root: Managed<T>,
	viewport_builder: egui::ViewportBuilder,
	egui_ctx: egui::Context,
	proxy: EventLoopProxy<UserEvent>,
	painter: Option<egui_wgpu::winit::Painter>,
	surfaced: Option<Surfaced>,
	error_dialog: crate::window::error_dialog::ErrorDialog,
	handle: Handle,
}

impl<T: RootWidget> WgpuRunner<T> {
	pub(crate) fn new(root: Managed<T>, options: ViewportBuilder, proxy: EventLoopProxy<UserEvent>) -> Self {
		Self {
			handle: root.handle.clone(),
			root,
			viewport_builder: options,
			proxy,
			painter: None,
			surfaced: None,
			egui_ctx: egui::Context::default(),
			error_dialog: crate::window::error_dialog::ErrorDialog::default(),
		}
	}

	fn redraw(&mut self, event_loop: &ActiveEventLoop) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};

		let raw_input = surfaced.egui_winit.take_egui_input(&surfaced.window);
		let output = self.egui_ctx.run_ui(raw_input, |ui| {
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

		for (_, egui::ViewportOutput { commands, .. }) in output.viewport_output {
			let mut actions_requested = Default::default();
			egui_winit::process_viewport_commands(&self.egui_ctx, &mut surfaced.viewport_info, commands, &surfaced.window, &mut actions_requested);
			for action in actions_requested {
				tracing::warn!("{action:?} is not supported by the wgpu backend");
			}
		}
		surfaced
			.egui_winit
			.handle_platform_output(&surfaced.window, output.platform_output);
		let primitives = self
			.egui_ctx
			.tessellate(output.shapes, output.pixels_per_point);
		let clear_color = self.root.clear_color();
		self.painter
			.as_mut()
			.expect("wgpu painter was not initialized")
			.paint_and_update_textures(
				egui::ViewportId::ROOT,
				output.pixels_per_point,
				[
					clear_color[0] as f32 / 255.0,
					clear_color[1] as f32 / 255.0,
					clear_color[2] as f32 / 255.0,
					clear_color[3] as f32 / 255.0,
				],
				&primitives,
				&output.textures_delta,
				Vec::new(),
				&surfaced.window,
			);
		surfaced.window.set_visible(true);

		event_loop.set_control_flow(if surfaced.repaint_delay.is_zero() {
			surfaced.window.request_redraw();
			winit::event_loop::ControlFlow::Poll
		} else if let Some(instant) = std::time::Instant::now().checked_add(surfaced.repaint_delay) {
			winit::event_loop::ControlFlow::WaitUntil(instant)
		} else {
			winit::event_loop::ControlFlow::Wait
		});
	}
}

impl<T: RootWidget> Runner for WgpuRunner<T> {
	fn is_visible(&self) -> bool {
		self.viewport_builder.visible.unwrap_or(true)
	}

	fn has_window(&self) -> bool {
		self.surfaced.is_some()
	}

	fn ensure_window(&mut self, event_loop: &ActiveEventLoop) {
		if self.surfaced.is_some() {
			return;
		}

		let attributes = egui_winit::create_winit_window_attributes(&self.egui_ctx, self.viewport_builder.clone());
		let window = Arc::new(
			event_loop
				.create_window(attributes)
				.expect("could not create winit window"),
		);
		egui_winit::apply_viewport_builder_to_window(&self.egui_ctx, &window, &self.viewport_builder);

		let painter = self.painter.get_or_insert_with(|| {
			pollster::block_on(egui_wgpu::winit::Painter::new(
				self.egui_ctx.clone(),
				egui_wgpu::WgpuConfiguration::default(),
				self.viewport_builder.transparent.unwrap_or(false),
				egui_wgpu::RendererOptions::default(),
			))
		});
		pollster::block_on(painter.set_window(egui::ViewportId::ROOT, Some(window.clone()))).expect("could not initialize wgpu surface");
		let render_state = painter
			.render_state()
			.expect("wgpu render state was not initialized");

		let egui_winit = egui_winit::State::new(
			self.egui_ctx.clone(),
			egui::ViewportId::ROOT,
			event_loop,
			None,
			event_loop.system_theme(),
			Some(render_state.device.limits().max_texture_dimension_2d as usize),
		);
		self.root.setup(&self.egui_ctx);

		let proxy = self.proxy.clone();
		self.egui_ctx.set_request_repaint_callback(move |info| {
			_ = proxy.send_event(UserEvent::RequestRepaint(info.delay));
		});

		window.set_visible(true);
		self.surfaced = Some(Surfaced {
			frame: Frame {
				handle: self.handle.clone(),
				render_state: Some(render_state),
				window: window.clone(),
				#[cfg(feature = "glow")]
				gl: None,
			},
			window,
			egui_winit,
			viewport_info: egui::ViewportInfo::default(),
			repaint_delay: Duration::MAX,
		});
		self.handle.visible.store(true, Ordering::Relaxed);
	}

	fn destroy_window(&mut self) {
		if self.surfaced.take().is_some()
			&& let Some(painter) = &mut self.painter
		{
			pollster::block_on(painter.set_window(egui::ViewportId::ROOT, None)).expect("could not release wgpu surface");
		}
		self.handle.visible.store(false, Ordering::Relaxed);
	}

	fn update(&mut self) {
		let span = tracing::span!(tracing::Level::INFO, "app_tick");
		let _enter = span.enter();
		if let Err(e) = self.root.update() {
			let (summary, details) = self.root.error(&e);
			self.error_dialog.emit(summary, details);
		}
		if let Some(surfaced) = &self.surfaced {
			surfaced.window.request_redraw();
		}
	}

	fn request_redraw(&self) {
		if let Some(surfaced) = &self.surfaced {
			surfaced.window.request_redraw();
		}
	}

	fn set_repaint_delay(&mut self, delay: Duration) {
		if let Some(surfaced) = &mut self.surfaced {
			surfaced.repaint_delay = delay;
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
		let Some(surfaced) = self.surfaced.as_mut() else {
			return;
		};
		if surfaced.window.id() != window_id {
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
		if let WindowEvent::Resized(size) = event
			&& let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
		{
			self.painter
				.as_mut()
				.expect("wgpu painter was not initialized")
				.on_window_resized(egui::ViewportId::ROOT, width, height);
		}
		let response = surfaced
			.egui_winit
			.on_window_event(&surfaced.window, &event);
		if response.repaint {
			surfaced.window.request_redraw();
		}
	}
}
