//! Demonstrates a small application composed from managed widgets.

use egelm::prelude::*;

#[derive(Debug, Widget)]
struct ExampleApp {
	storage: Storage,
	input: String,
}

impl Widget for ExampleApp {
	type Message = ();
	type Error = String;
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			ui.text_edit_multiline(&mut self.input);
			if ui.button("Save").clicked() {
				ctx.emit(());
			}
		});
	}

	fn update(&mut self, _msg: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		self.storage
			.set("input", &self.input)
			.map_err(|e| format!("{e}"))?;
		self.storage.flush().map_err(|e| format!("{e}"))?;
		Ok(())
	}
}

impl RootWidget for ExampleApp {
	fn error(&mut self, err: &Self::Error) -> (String, Option<String>) {
		(err.clone(), None)
	}
}

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt::init();

	let app = App::try_new_factory(
		ViewportBuilder::default()
			.with_title("Simple Example")
			.with_app_id("dev.egelm.StorageExample"),
		|ctx| {
			Ok::<ExampleApp, egelm::error::Error>(ExampleApp {
				input: ctx.storage.get("input")?.unwrap_or_default(),
				storage: ctx.storage.clone(),
			})
		},
	)
	.unwrap();

	app.run().unwrap();
}
