//! Demonstrates a small application composed from managed widgets.

#![cfg_attr(not(feature = "storage"), allow(unused))]

use egelm::prelude::*;

#[cfg(feature = "storage")]
#[derive(Debug, Widget)]
struct ExampleApp {
	input: String,
}

#[cfg(feature = "storage")]
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

	fn update(&mut self, _msg: Self::Message, _handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		ctx.storage()
			.set("input", &self.input)
			.map_err(|e| format!("{e}"))?;
		ctx.storage().flush().map_err(|e| format!("{e}"))?;
		Ok(())
	}
}

#[cfg(feature = "storage")]
impl RootWidget for ExampleApp {
	fn error(&mut self, err: &Self::Error) -> (String, Option<String>) {
		(err.clone(), None)
	}
}

#[cfg(feature = "storage")]
#[tokio::main]
async fn main() {
	tracing_subscriber::fmt::init();

	let app = App::try_new_factory(
		ViewportBuilder::default()
			.with_title("Simple Example")
			.with_app_id("dev.egelm.StorageExample"),
		|ctx| {
			Ok::<ExampleApp, egelm::error::Error>(ExampleApp {
				input: ctx.storage().get("input")?.unwrap_or_default(),
			})
		},
	)
	.unwrap();

	app.run().unwrap();
}

#[cfg(not(feature = "storage"))]
fn main() {
	panic!("You must have the `storage` feature enabled for this example.");
}
