//! Demonstrates routing widget errors to the root error dialog.

mod common;

use egelm::prelude::*;

#[derive(Widget)]
struct InnerWidget;

impl Widget for InnerWidget {
	type Message = ();
	type Error = String;
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, _frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui
				.button("Create error from inner widget's view()")
				.clicked()
			{
				ctx.error("Error from view()".to_string());
			}
			if ui
				.button("Create error from inner widget's update()")
				.clicked()
			{
				ctx.emit(());
			}
		});
	}

	fn update(&mut self, _msg: Self::Message, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		Err("Error from update()".to_string())
	}
}

#[derive(Widget)]
struct ExampleApp {
	inner: Managed<InnerWidget>,
}

impl Widget for ExampleApp {
	type Message = ();
	type Error = String;
	type Output = ();

	fn view(&mut self, ui: &mut egui::Ui, frame: &mut Frame, ctx: &Context<Self>) {
		ui.vertical_centered(|ui| {
			if ui.button("Create error from view()").clicked() {
				ctx.error("Error from view()".to_string());
			}
			if ui.button("Create error from update()").clicked() {
				ctx.emit(());
			}
			if ui.button("Create error 2 errors from update()").clicked() {
				ctx.emit(());
				ctx.emit(());
			}
		});

		self.inner.render(ui, frame);
	}

	fn update(&mut self, _msg: Self::Message, _ctx: &Context<Self>) -> Result<(), Self::Error> {
		Err("Error from update()".to_string())
	}
}

impl RootWidget for ExampleApp {
	fn error(&mut self, err: &Self::Error) -> (String, Option<String>) {
		(err.clone(), Some("this can be a long description of the error".to_string()))
	}
}

#[egelm::main]
async fn main() {
	common::init_tracing();

	let app = App::new_factory(ViewportBuilder::default().with_title("Simple Example"), |ctx| ExampleApp {
		inner: ctx.manage(
			None,
			ctx.error_sender()
				.map(|s| format!("from inner widget: {s}")),
			InnerWidget,
		),
	});

	app.run().await.unwrap();
}
