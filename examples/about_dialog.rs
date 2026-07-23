//! Demonstrates opening the reusable about dialog.

use egelm::prelude::*;
use egelm::widgets::prelude::*;

#[derive(Debug)]
enum Message {
	Open,
}

#[derive(Debug, Widget)]
struct ExampleApp {
	about_dialog: AboutDialog,
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
		});

		self.about_dialog.render(ui, frame);
	}

	fn update(&mut self, msg: Self::Message, _handle: &Handle, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Open => self.about_dialog.open(),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {
	fn setup(&mut self, ctx: &egui::Context) {
		ctx.include_bytes("bytes://empty.svg", &[]);
	}
}

#[tokio::main]
async fn main() {
	let app = App::new(ExampleApp {
		about_dialog: AboutDialog::new(AboutDialogSettings {
			name: "Example app".to_string(),
			description: "An example for egelm".to_string(),
			description_long: "This is a longer description for this app".to_string(),
			developer: "Sam Jones".to_string(),
			version: env!("CARGO_PKG_VERSION").to_string(),
			issues_url: "https://codeberg.org/Land/egelm/issues".to_string(),
			website_url: "https://codeberg.org/Land/egelm".to_string(),
			icon_uri: "bytes://empty.svg".to_string(),
			license_text: include_str!("../LICENSE").to_string(),
			license_name: "MIT License".to_string(),
			license_year: 2026,
		}),
	});

	app.run(ViewportBuilder::default().with_title("Simple Example"))
		.unwrap();
}
