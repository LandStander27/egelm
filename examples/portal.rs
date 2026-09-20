//! Demonstrates a small application using the xdg-desktop-portal.

#![cfg_attr(not(feature = "portal"), allow(unused))]

use egelm::prelude::*;

enum Message {
	Open,
	OpenConfirmed(String),
}

#[cfg(feature = "portal")]
#[derive(Widget, Default)]
struct ExampleApp {
	path: String,
}

#[cfg(feature = "portal")]
impl Widget for ExampleApp {
	type Message = Message;
	type Error = String;
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui.button("Open file").clicked() {
				ctx.emit(Message::Open);
			}

			ui.label(&self.path);
		});
	}

	fn update(&mut self, msg: Self::Message, ctx: &Context<Self>) -> Result<(), Self::Error> {
		match msg {
			Message::Open => {
				ctx.spawn(async move |ctx| {
					let response = ctx
						.portal(egelm::portal::OpenFile::default())
						.await
						.map_err(|e| e.to_string())?;
					Ok(Message::OpenConfirmed(response[0].display().to_string()))
				});
			}
			Message::OpenConfirmed(s) => self.path = s,
		}

		Ok(())
	}
}

#[cfg(feature = "portal")]
impl RootWidget for ExampleApp {
	fn error(&mut self, err: &Self::Error) -> (String, Option<String>) {
		(err.clone(), None)
	}
}

#[cfg(feature = "portal")]
#[egelm::main]
async fn main() {
	let app = App::new_with_options(ViewportBuilder::default().with_title("XDG Desktop Portal example"), ExampleApp::default());

	app.run().unwrap();
}

#[cfg(not(feature = "portal"))]
fn main() {
	panic!("this example requires the feature `portal`.");
}
