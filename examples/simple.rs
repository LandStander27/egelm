//! Demonstrates a small application composed from managed widgets.

mod common;

use egelm::prelude::*;

#[derive(Widget, Default)]
struct InputDialog {
	input: String,
	open: bool,
}

impl InputDialog {
	pub fn open(&mut self) {
		self.open = true;
	}
}

impl Widget for InputDialog {
	type Error = ();
	type Message = ();
	type Output = String;

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		if !self.open {
			return;
		}

		Modal::new("input_dialog".into()).show(ui.ctx(), |ui| {
			ui.vertical_centered(|ui| {
				let response = ui.add(TextEdit::singleline(&mut self.input));
				if ui.button("Confirm").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
					ctx.output(self.input.take());
					self.open = false;
				}
			});
		});
	}
}

enum Message {
	Confirmed(String),
	Open,
}

#[derive(Widget)]
struct ExampleApp {
	input_dialog: Managed<InputDialog>,
	input: Option<String>,
}

impl Widget for ExampleApp {
	type Message = Message;
	type Error = ();
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui.button("Open dialog").clicked() {
				ctx.emit(Message::Open);
			}

			if let Some(s) = &self.input {
				ui.label(format!("Got: {s}"));
			}
		});

		self.input_dialog.render(ui, frame);
	}

	fn update(&mut self, msg: Self::Message, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Open => self.input_dialog.open(),
			Message::Confirmed(s) => self.input = Some(s),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[egelm::main] // Equivalent to tokio::main
async fn main() {
	common::init_tracing();

	let app = App::new_factory(ViewportBuilder::default().with_title("Simple Example"), |ctx| ExampleApp {
		input: None,
		input_dialog: ctx.manage(ctx.input_sender().map(Message::Confirmed), ctx.error_sender(), InputDialog::default()),
	});

	app.run().await.unwrap();
}
