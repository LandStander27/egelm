//! Demonstrates routing output from multiple managed child widgets.

use egelm::prelude::*;

#[derive(Debug)]
enum CounterMessage {
	Increment,
}

#[derive(Debug, Widget)]
struct Counter {
	label: &'static str,
	value: u32,
}

impl Widget for Counter {
	type Message = CounterMessage;
	type Error = ();
	type Output = (&'static str, u32);

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		if ui
			.button(format!("{}: {}", self.label, self.value))
			.clicked()
		{
			ctx.emit(CounterMessage::Increment);
		}
	}

	fn update(&mut self, message: Self::Message, _handle: &Handle, ctx: &Context<Self>) -> Result<(), Self::Error> {
		match message {
			CounterMessage::Increment => {
				self.value += 1;
				ctx.output((self.label, self.value));
			}
		}

		Ok(())
	}
}

#[derive(Debug)]
enum Message {
	CounterChanged(&'static str, u32),
}

#[derive(Debug, Widget)]
struct ExampleApp {
	first: Managed<Counter>,
	second: Managed<Counter>,
	last_change: Option<(&'static str, u32)>,
}

impl Widget for ExampleApp {
	type Message = Message;
	type Error = ();
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, _ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			ui.heading("Managed children");
			ui.horizontal(|ui| {
				self.first.render(ui, frame);
				self.second.render(ui, frame);
			});

			if let Some((label, value)) = self.last_change {
				ui.label(format!("{label} changed to {value}"));
			}
		});
	}

	fn update(&mut self, message: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		match message {
			Message::CounterChanged(label, value) => self.last_change = Some((label, value)),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[tokio::main]
async fn main() {
	let app = App::new_factory(ViewportBuilder::default().with_title("Multiple Children Example"), |ctx| {
		let output = ctx
			.ctx
			.input_sender()
			.map(|(label, value)| Message::CounterChanged(label, value));

		ExampleApp {
			first: Managed::new(output.clone(), ctx.ctx.error_sender(), ctx.handle, Counter { label: "First", value: 0 }),
			second: Managed::new(output, ctx.ctx.error_sender(), ctx.handle, Counter { label: "Second", value: 0 }),
			last_change: None,
		}
	});

	app.run().unwrap();
}
