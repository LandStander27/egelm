//! A reusable modal containing application details and legal information.

use crate::prelude::*;
use crate::widgets::prelude::*;

#[derive(Debug, PartialEq, Default)]
enum AboutTab {
	#[default]
	Details,
	Legal,
}

/// Content displayed by an [`AboutDialog`].
///
/// # Examples
///
/// ```
/// use egelm::window::about_dialog::AboutDialogSettings;
///
/// let settings = AboutDialogSettings {
///     name: "My App".into(),
///     icon_uri: "file:///tmp/icon.png".into(),
///     version: "1.0.0".into(),
///     developer: "Example Developer".into(),
///     website_url: "https://example.com".into(),
///     issues_url: "https://example.com/issues".into(),
///     description: "A short description".into(),
///     description_long: "A longer description of the application.".into(),
///     license_year: 2026,
///     license_name: "MIT".into(),
///     license_text: "Permission is hereby granted...".into(),
/// };
/// ```
#[derive(Debug)]
pub struct AboutDialogSettings {
	/// The application name shown in the dialog heading.
	pub name: String,
	/// The URI of the application icon.
	pub icon_uri: String,
	/// The application version string.
	pub version: String,
	/// The person or organization that develops the application.
	pub developer: String,
	/// The application's main website URL.
	pub website_url: String,
	/// The URL where users can report issues.
	pub issues_url: String,
	/// A short application tagline shown beneath its name.
	pub description: String,
	/// A longer application description shown on the details tab.
	pub description_long: String,
	/// The copyright year shown on the legal tab.
	pub license_year: u32,
	/// The name of the application's license.
	pub license_name: String,
	/// The full license text shown on the legal tab.
	pub license_text: String,
}

/// A modal about dialog with details and legal tabs.
///
/// The dialog starts closed. Call [`open`](Self::open) in response to a user
/// action, then render it as a [`LeafWidget`].
#[derive(Debug)]
pub struct AboutDialog {
	open: bool,
	tab: AboutTab,
	settings: AboutDialogSettings,
}

impl AboutDialog {
	/// Creates a closed dialog containing `settings`.
	///
	/// # Examples
	///
	/// ```ignore
	/// use egelm::window::prelude:*;
	///
	/// let dialog = AboutDialog::new(AboutDialogSettings {
	///     name: "My App".into(),
	///     icon_uri: String::new(),
	///     version: "1.0.0".into(),
	///     developer: "Example Developer".into(),
	///     website_url: "https://example.com".into(),
	///     issues_url: "https://example.com/issues".into(),
	///     description: "An example".into(),
	///     description_long: "An example application.".into(),
	///     license_year: 2026,
	///     license_name: "MIT License".into(),
	///     license_text: include_str!("../LICENSE").to_string(),
	/// });
	/// ```
	pub fn new(settings: AboutDialogSettings) -> Self {
		Self {
			open: false,
			tab: AboutTab::default(),
			settings,
		}
	}

	/// Opens the dialog for display on its next render.
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
