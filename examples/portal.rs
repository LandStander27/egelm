//! Demonstrates a small application using the xdg-desktop-portal.

#![cfg_attr(not(feature = "portal"), allow(unused))]

mod common;

use egelm::prelude::*;

enum Message {
	ShortcutPressed,
	Open,
	OpenConfirmed(String),
	OpenURI,
	Void,
}

#[cfg(feature = "portal")]
#[derive(Widget, Default)]
struct ExampleApp {
	path: String,
	amount: u64,
}

#[cfg(feature = "portal")]
impl Widget for ExampleApp {
	type Message = Message;
	type Error = String;
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			ui.label(format!("Shortcut pressed {} times", self.amount));

			if ui.button("Open file").clicked() {
				ctx.emit(Message::Open);
			}

			ui.label(&self.path);

			if !self.path.is_empty() && ui.button("Open this file with default program").clicked() {
				ctx.emit(Message::OpenURI);
			}
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
			Message::ShortcutPressed => self.amount += 1,
			Message::OpenURI => {
				ctx.spawn({
					let path = self.path.clone();
					async move |ctx| {
						ctx.portal(egelm::portal::OpenURI::file(path))
							.await
							.map_err(|e| e.to_string())?;
						Ok(Message::Void)
					}
				});
			}
			Message::Void => {}
		}

		Ok(())
	}

	fn init(&mut self, ctx: &Context<Self>) {
		ctx.spawn(async move |ctx| {
			let mut session = ctx
				.portal(egelm::portal::GlobalShortcuts::new().shortcut("shortcut", "Example shortcut"))
				.await
				.map_err(|e| e.to_string())?;

			while let Some(id) = session.next_activated().await {
				assert_eq!(id, "shortcut");

				ctx.emit(Message::ShortcutPressed);
			}

			Ok(Message::Void)
		});
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
	common::init_tracing();

	let app = App::new_with_options(
		ViewportBuilder::default()
			.with_title("XDG Desktop Portal example")
			.with_app_id("dev.egelm.PortalsExample"),
		ExampleApp::default(),
	);

	app.run().await.unwrap();
}

#[cfg(not(feature = "portal"))]
fn main() {
	panic!("this example requires the feature `portal`.");
}
