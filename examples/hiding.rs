//! Demonstrates hiding and restoring the native application window.

use egelm::prelude::*;

#[derive(Debug)]
enum Message {
	Close,
	Show,
}

#[derive(Debug, Widget)]
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

	fn update(&mut self, msg: Self::Message, handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Close => {
				handle.hide();
				ctx.spawn(|_ctx| async move {
					tokio::time::sleep(std::time::Duration::from_secs(5)).await;
					Ok(Message::Show)
				});
			}
			Message::Show => handle.show(),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[egelm::main]
async fn main() {
	let app = App::new_with_options(ViewportBuilder::default().with_title("Simple Example"), ExampleApp);

	app.run().unwrap();
}
