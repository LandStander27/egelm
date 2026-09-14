//! Demonstrates a small application that shows how toasts work.

use egelm::prelude::*;
use egelm::widgets::*;

#[derive(Debug)]
enum Message {
	Info,
	Success,
	Warning,
	Error,
}

#[derive(Debug, Widget)]
struct ExampleApp;

impl Widget for ExampleApp {
	type Message = Message;
	type Error = ();
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui.button("Add info toast").clicked() {
				ctx.emit(Message::Info);
			}
			if ui.button("Add success toast").clicked() {
				ctx.emit(Message::Success);
			}
			if ui.button("Add warning toast").clicked() {
				ctx.emit(Message::Warning);
			}
			if ui.button("Add error toast").clicked() {
				ctx.emit(Message::Error);
			}
		});
	}

	fn update(&mut self, msg: Self::Message, _handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Info => ctx.toast(
				Toast::new("Some info")
					.info()
					.body("fyi...")
					.action("Understood", |_ctx| println!("user understands the information!")),
			),
			Message::Success => ctx.toast(Toast::new("Some successful thing").success().body(":)")),
			Message::Warning => ctx.toast(
				Toast::new("Some warning")
					.warning()
					.body("...?")
					.duration(std::time::Duration::from_secs(30)),
			),
			Message::Error => ctx.toast(Toast::new("Some error").error().body(":(").sticky()),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[tokio::main]
async fn main() {
	let app = App::new_with_options(ViewportBuilder::default().with_title("Simple Example"), ExampleApp);

	app.run().unwrap();
}
