//! Example using `egui`'s demo.

use egelm::prelude::*;

#[derive(Default)]
struct ExampleApp {
	demo_windows: egui_demo_lib::DemoWindows,
}

impl std::fmt::Debug for ExampleApp {
	fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Ok(())
	}
}

impl LeafWidget for ExampleApp {
	fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
		self.demo_windows.ui(ui);
		self.demo_windows.logic(ui.ctx());
	}
}

impl RootWidget for ExampleApp {
	fn setup(&mut self, ctx: &egui::Context) {
		egui_extras::install_image_loaders(ctx);
	}
}

#[tokio::main]
async fn main() {
	App::new(ExampleApp::default()).run().unwrap();
}
