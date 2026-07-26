//! Demonstrates running background work and returning its result as a message.

use std::time::Duration;

use egelm::prelude::*;

#[derive(Debug)]
enum Message {
	Load,
	Loaded(String),
}

#[derive(Debug, Widget)]
struct ExampleApp {
	loading: bool,
	result: Option<String>,
}

impl Widget for ExampleApp {
	type Message = Message;
	type Error = ();
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui
				.add_enabled(!self.loading, egui::Button::new("Load in background"))
				.clicked()
			{
				ctx.emit(Message::Load);
			}

			if self.loading {
				ui.spinner();
			} else if let Some(result) = &self.result {
				ui.label(result);
			}
		});
	}

	fn update(&mut self, message: Self::Message, _handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		match message {
			Message::Load => {
				self.loading = true;
				self.result = None;
				ctx.spawn(|_ctx| async move {
					tokio::time::sleep(Duration::from_secs(2)).await;
					Ok(Message::Loaded("Background task finished".to_string()))
				});
			}
			Message::Loaded(result) => {
				self.loading = false;
				self.result = Some(result);
			}
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[tokio::main]
async fn main() {
	App::new(ExampleApp { loading: false, result: None })
		.run(ViewportBuilder::default().with_title("Async Task Example"))
		.unwrap();
}
