//! Demonstrates a small application for Android.

#![allow(unused)]

use egelm::prelude::*;

#[derive(Debug, Widget, Default)]
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

#[derive(Debug)]
enum Message {
	Confirmed(String),
	Open,
}

#[derive(Debug, Widget)]
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

	fn update(&mut self, msg: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Open => self.input_dialog.open(),
			Message::Confirmed(s) => self.input = Some(s),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: AndroidApp) {
	android_app.set_window_flags(
		WindowManagerFlags::FORCE_NOT_FULLSCREEN,
		WindowManagerFlags::FULLSCREEN | WindowManagerFlags::LAYOUT_IN_SCREEN | WindowManagerFlags::LAYOUT_NO_LIMITS,
	);

	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();

	let _guard = runtime.enter(); // Needed for calling Context::spawn

	let app = App::new_factory(|ctx, handle| ExampleApp {
		input: None,
		input_dialog: Managed::new(ctx.input_sender().map(Message::Confirmed), ctx.error_sender(), handle, InputDialog::default()),
	});

	app.run_android(android_app, ViewportBuilder::default().with_title("Simple Example"))
		.unwrap();
}

#[cfg(all(target_os = "linux", not(target_os = "android")))]
fn main() {
	panic!("This example is only for Android. Please run it on an Android device or emulator.");
}
