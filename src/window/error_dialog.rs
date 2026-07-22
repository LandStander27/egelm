use crate::prelude::*;
use crate::widgets::prelude::*;

#[derive(Debug)]
struct QueuedError {
	summary: String,
	details: Option<String>,
}

#[derive(Default, Debug)]
pub struct ErrorDialog {
	errors: std::collections::VecDeque<QueuedError>,
}

impl ErrorDialog {
	pub fn emit(&mut self, summary: String, details: Option<String>) {
		self.errors.push_back(QueuedError { summary, details });
	}
}

impl LeafWidget for ErrorDialog {
	fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
		if self.errors.is_empty() {
			return;
		}

		egui::Modal::new(egui::Id::new("error_dialog")).show(ui, |ui| {
			ui.set_width(460.0);
			ui.set_max_height(520.0);

			ui.vertical_centered(|ui| {
				ui.add_space(12.0);
				ui.add(
					Label::new(
						RichText::new(emoji("warning"))
							.size(40.0)
							.color(Color32::from_rgb(0xe0, 0x1b, 0x24)),
					)
					.selectable(false),
				);
				ui.add_space(8.0);
				ui.label(RichText::new("An error occurred").size(18.0).strong());
				ui.label(
					RichText::new(if self.errors.len() > 1 {
						format!("1 of {} errors", self.errors.len())
					} else {
						String::new()
					})
					.size(11.5)
					.weak(),
				);
			});

			ui.separator();
			ui.add_space(10.0);

			let summary = self
				.errors
				.front()
				.map(|e| e.summary.as_str())
				.unwrap_or_default();

			egui::Frame::new()
				.fill(ui.visuals().extreme_bg_color)
				.corner_radius(8)
				.inner_margin(12)
				.show(ui, |ui| {
					ui.label(RichText::new(summary).monospace().size(12.5));
				});

			ui.add_space(10.0);

			if let Some(details) = self.errors.front()
				&& let Some(details) = &details.details
			{
				ui.collapsing("Details", |ui| {
					egui::Frame::new()
						.fill(ui.visuals().extreme_bg_color)
						.corner_radius(8)
						.inner_margin(10)
						.show(ui, |ui| {
							ScrollArea::both().show(ui, |ui| {
								ui.add(
									Label::new(RichText::new(details).monospace())
										.wrap_mode(TextWrapMode::Wrap)
										.selectable(false),
								);
							})
						});

					ui.add_space(6.0);
				});

				ui.separator();
			}

			ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
				if ui
					.button(if self.errors.len() > 1 {
						"Dismiss"
					} else {
						"Close"
					})
					.clicked()
				{
					self.errors.pop_front();
				}

				if let Some(details) = self.errors.front().and_then(|e| e.details.clone())
					&& ui.button("Copy Details").clicked()
				{
					ui.copy_text(details);
				}
			});
		});
	}
}
