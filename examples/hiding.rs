//! Demonstrates hiding and restoring the native application window.

mod common;

use egelm::prelude::*;

enum Message {
	Close,
	Show,
}

#[derive(Widget)]
struct ExampleApp;

impl Widget for ExampleApp {
	type Message = Message;
	type Error = ();
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui.button("Close window for 5 seconds").clicked() {
				ctx.emit(Message::Close);
			}
		});
	}

	fn update(&mut self, msg: Self::Message, ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Close => {
				ctx.handle().hide();
				ctx.spawn(|_ctx| async move {
					tokio::time::sleep(std::time::Duration::from_secs(5)).await;
					Ok(Message::Show)
				});
			}
			Message::Show => ctx.handle().show(),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[egelm::main]
async fn main() {
	common::init_tracing();

	let app = App::new_with_options(
		ViewportBuilder::default()
			.with_title("Simple Example")
			.with_app_id("dev.egelm.PortalsExample"),
		ExampleApp,
	);

	app.run().await.unwrap();
}
