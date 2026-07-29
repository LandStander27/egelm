//! `wgpu` rendering resources.

use super::{Backend, Handle, RenderFrame};
use crate::prelude::*;

use std::num::NonZeroU32;
use std::sync::Arc;

use egui_winit::winit;
use winit::event_loop::ActiveEventLoop;

#[derive(Default)]
pub(crate) struct WgpuBackend {
	painter: Option<egui_wgpu::winit::Painter>,
	surface: Option<WgpuSurface>,
}

impl WgpuBackend {
	fn painter(&self) -> &egui_wgpu::winit::Painter {
		self.painter
			.as_ref()
			.expect("wgpu painter was not initialized")
	}

	fn painter_mut(&mut self) -> &mut egui_wgpu::winit::Painter {
		self.painter
			.as_mut()
			.expect("wgpu painter was not initialized")
	}

	fn surface(&self) -> &WgpuSurface {
		self.surface
			.as_ref()
			.expect("wgpu surface was not initialized")
	}
}

struct WgpuSurface {
	window: Arc<winit::window::Window>,
}

impl Backend for WgpuBackend {
	fn name(&self) -> &'static str {
		"wgpu"
	}

	fn create_surface(&mut self, egui_ctx: &egui::Context, event_loop: &ActiveEventLoop, viewport_builder: &ViewportBuilder) {
		let attributes = egui_winit::create_winit_window_attributes(egui_ctx, viewport_builder.clone());
		let window = Arc::new(
			event_loop
				.create_window(attributes)
				.expect("could not create winit window"),
		);
		egui_winit::apply_viewport_builder_to_window(egui_ctx, &window, viewport_builder);

		let painter = self.painter.get_or_insert_with(|| {
			pollster::block_on(egui_wgpu::winit::Painter::new(
				egui_ctx.clone(),
				egui_wgpu::WgpuConfiguration::default(),
				viewport_builder.transparent.unwrap_or(false),
				egui_wgpu::RendererOptions::default(),
			))
		});
		pollster::block_on(painter.set_window(egui::ViewportId::ROOT, Some(window.clone()))).expect("could not initialize wgpu surface");
		self.surface = Some(WgpuSurface { window });
	}

	fn window(&self) -> &Arc<winit::window::Window> {
		&self.surface().window
	}

	fn max_texture_side(&self) -> usize {
		self.painter()
			.render_state()
			.expect("wgpu render state was not initialized")
			.device
			.limits()
			.max_texture_dimension_2d as usize
	}

	fn frame(&self, handle: Handle) -> Frame {
		Frame {
			handle,
			render_state: self.painter().render_state(),
			window: self.surface().window.clone(),
			#[cfg(feature = "glow")]
			gl: None,
		}
	}

	fn prepare_frame(&mut self, events: &mut Vec<egui::Event>) {
		self.painter().handle_screenshots(events);
	}

	fn paint(&mut self, frame: RenderFrame<'_>) {
		let window = self.surface().window.clone();
		self.painter_mut().paint_and_update_textures(
			egui::ViewportId::ROOT,
			frame.pixels_per_point,
			frame.clear_color,
			frame.primitives,
			frame.textures_delta,
			frame.screenshots,
			&window,
		);
	}

	fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
		if let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
			self.painter_mut()
				.on_window_resized(egui::ViewportId::ROOT, width, height);
		}
	}

	fn destroy_surface(&mut self) {
		pollster::block_on(self.painter_mut().set_window(egui::ViewportId::ROOT, None)).expect("could not release wgpu surface");
		self.surface = None;
	}
}
