//! Demonstrates composing a root application from rendering-only leaf widgets.

use egelm::prelude::*;

#[derive(Debug, Default)]
struct ClickCounter {
	count: u32,
}

impl LeafWidget for ClickCounter {
	fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
		if ui.button(format!("Clicked {} times", self.count)).clicked() {
			self.count += 1;
		}
	}
}

#[derive(Debug, Default)]
struct ExampleApp {
	counter: ClickCounter,
}

impl LeafWidget for ExampleApp {
	fn render(&mut self, ui: &mut egui::Ui, frame: &mut Frame) {
		ui.vertical_centered(|ui| {
			ui.heading("A simple leaf widget");
			self.counter.render(ui, frame);
		});
	}
}

impl RootWidget for ExampleApp {}

#[tokio::main]
async fn main() {
	App::new(ExampleApp::default()).run().unwrap();
}
