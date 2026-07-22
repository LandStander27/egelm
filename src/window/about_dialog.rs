use crate::prelude::*;
use crate::widgets::prelude::*;

#[derive(Debug, PartialEq, Default)]
enum AboutTab {
	#[default]
	Details,
	Legal,
}

#[derive(Debug)]
pub struct AboutDialogSettings {
	pub name: String,
	pub icon_uri: String,
	pub version: String,
	pub developer: String,
	pub website_url: String,
	pub issues_url: String,
	pub description: String,
	pub description_long: String,
	pub license_year: u32,
	pub license_name: String,
	pub license_text: String,
}

#[derive(Debug)]
pub struct AboutDialog {
	open: bool,
	tab: AboutTab,
	settings: AboutDialogSettings,
}

impl AboutDialog {
	pub fn new(settings: AboutDialogSettings) -> Self {
		Self {
			open: false,
			tab: AboutTab::default(),
			settings,
		}
	}

	pub fn open(&mut self) {
		self.open = true;
	}

	fn details_tab(&self, ui: &mut egui::Ui) {
		ui.label(format!("Developed by {}", self.settings.developer));
		ui.add_space(6.0);
		ui.hyperlink_to("Website", &self.settings.website_url);
		ui.hyperlink_to("Report an issue", &self.settings.issues_url);
		ui.add_space(10.0);
		ui.label(
			RichText::new(&self.settings.description_long)
				.size(11.5)
				.weak(),
		);
	}

	fn legal_tab(&mut self, ui: &mut egui::Ui) {
		ui.label(RichText::new(&self.settings.license_name).strong());
		ui.add_space(4.0);
		ui.label(format!("© {} {}", self.settings.license_year, self.settings.developer));
		ui.add_space(10.0);
		ui.add(
			TextEdit::multiline(&mut self.settings.license_text)
				.desired_width(ui.available_width())
				.font(TextStyle::Small)
				.interactive(false),
		);
	}
}

impl LeafWidget for AboutDialog {
	fn render(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
		if !self.open {
			return;
		}

		Modal::new(egui::Id::new("about_dialog")).show(ui, |ui| {
			ui.set_width(420.0);
			ui.set_max_height(560.0);

			// Header: icon, name, tagline, version — mirrors AdwAboutDialog's header.
			ui.vertical_centered(|ui| {
				ui.add_space(12.0);
				ui.add(
					Image::from_uri(&self.settings.icon_uri)
						.fit_to_exact_size(egui::vec2(64.0, 64.0))
						.maintain_aspect_ratio(true),
				);
				ui.add_space(8.0);
				ui.label(RichText::new(&self.settings.name).size(20.0).strong());
				ui.label(RichText::new(&self.settings.description).size(13.0).weak());
				ui.add_space(2.0);
				ui.label(
					RichText::new(format!("Version {}", self.settings.version))
						.size(12.0)
						.weak(),
				);
				ui.add_space(12.0);
			});

			ui.separator();

			// Tab bar, like AdwAboutDialog's internal navigation.
			ui.horizontal(|ui| {
				ui.selectable_value(&mut self.tab, AboutTab::Details, "Details");
				ui.selectable_value(&mut self.tab, AboutTab::Legal, "Legal");
			});

			ui.separator();
			ui.add_space(6.0);

			ScrollArea::vertical()
				.max_height(300.0)
				.show(ui, |ui| match self.tab {
					AboutTab::Details => self.details_tab(ui),
					AboutTab::Legal => self.legal_tab(ui),
				});

			ui.add_space(8.0);
			ui.separator();
			ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
				if ui.button("Close").clicked() {
					self.open = false;
				}
			});
		});
	}
}
