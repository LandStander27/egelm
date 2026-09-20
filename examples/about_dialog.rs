//! Demonstrates opening the reusable about dialog.

mod common;

use egelm::prelude::*;
use egelm::widgets::AboutDialog;

enum Message {
	Open,
}

#[derive(Widget)]
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

	fn update(&mut self, msg: Self::Message, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Open => self.about_dialog.open(),
		}

		Ok(())
	}
}

impl RootWidget for ExampleApp {
	fn setup(&mut self, ctx: &egui::Context) {
		egui_extras::install_image_loaders(ctx);
		ctx.include_bytes("bytes://blank.svg", br#"<svg height="100" width="100" xmlns="http://www.w3.org/2000/svg"></svg>"#);
	}
}

#[egelm::main]
async fn main() {
	common::init_tracing();

	let app = App::new_with_options(
		ViewportBuilder::default().with_title("Simple Example"),
		ExampleApp {
			about_dialog: AboutDialog::new("Example app")
				.description("An example for egelm")
				.description_long("This is a longer description for this app")
				.developer("Sam Jones")
				.version(env!("CARGO_PKG_VERSION"))
				.issues_url("https://codeberg.org/Land/egelm/issues")
				.website_url("https://codeberg.org/Land/egelm")
				.icon_uri("bytes://blank.svg")
				.license_text(include_str!("../LICENSE"))
				.license_name("MIT License")
				.license_year(2026),
		},
	);

	app.run().await.unwrap();
}
