//! A reusable modal containing application details and legal information.

use crate::prelude::*;

#[derive(Debug, PartialEq, Default)]
enum AboutTab {
	#[default]
	Details,
	Legal,
}

/// A modal about dialog with optional application details and legal information.
///
/// The application name is required; all other content can be added with the
/// chainable setters. The dialog starts closed. Call [`open`](Self::open) in
/// response to a user action, then render it as a [`LeafWidget`].
///
/// # Examples
///
/// ```
/// use egelm::window::about_dialog::AboutDialog;
///
/// let dialog = AboutDialog::new("My App")
///     .version("1.0.0")
///     .developer("Example Developer")
///     .website_url("https://example.com")
///     .description("A short description");
/// ```
#[derive(Debug)]
pub struct AboutDialog {
	open: bool,
	tab: AboutTab,
	name: String,
	icon_uri: Option<String>,
	version: Option<String>,
	developer: Option<String>,
	website_url: Option<String>,
	issues_url: Option<String>,
	description: Option<String>,
	description_long: Option<String>,
	license_year: Option<u32>,
	license_name: Option<String>,
	license_text: Option<String>,
}

impl AboutDialog {
	/// Creates a closed dialog for the application named `name`.
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			open: false,
			tab: AboutTab::default(),
			name: name.into(),
			icon_uri: None,
			version: None,
			developer: None,
			website_url: None,
			issues_url: None,
			description: None,
			description_long: None,
			license_year: None,
			license_name: None,
			license_text: None,
		}
	}

	/// Sets the URI of the application icon.
	pub fn icon_uri(mut self, icon_uri: impl Into<String>) -> Self {
		self.icon_uri = Some(icon_uri.into());
		self
	}

	/// Sets the application version string.
	pub fn version(mut self, version: impl Into<String>) -> Self {
		self.version = Some(version.into());
		self
	}

	/// Sets the person or organization that develops the application.
	pub fn developer(mut self, developer: impl Into<String>) -> Self {
		self.developer = Some(developer.into());
		self
	}

	/// Sets the application's main website URL.
	pub fn website_url(mut self, website_url: impl Into<String>) -> Self {
		self.website_url = Some(website_url.into());
		self
	}

	/// Sets the URL where users can report issues.
	pub fn issues_url(mut self, issues_url: impl Into<String>) -> Self {
		self.issues_url = Some(issues_url.into());
		self
	}

	/// Sets the short application tagline shown beneath its name.
	pub fn description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Sets the longer application description shown with the details.
	pub fn description_long(mut self, description_long: impl Into<String>) -> Self {
		self.description_long = Some(description_long.into());
		self
	}

	/// Sets the copyright year shown with the legal information.
	pub fn license_year(mut self, license_year: u32) -> Self {
		self.license_year = Some(license_year);
		self
	}

	/// Sets the name of the application's license.
	pub fn license_name(mut self, license_name: impl Into<String>) -> Self {
		self.license_name = Some(license_name.into());
		self
	}

	/// Sets the full license text.
	pub fn license_text(mut self, license_text: impl Into<String>) -> Self {
		self.license_text = Some(license_text.into());
		self
	}

	/// Opens the dialog for display on its next render.
	pub fn open(&mut self) {
		self.open = true;
	}

	fn has_details(&self) -> bool {
		self.developer.is_some() || self.website_url.is_some() || self.issues_url.is_some() || self.description_long.is_some()
	}

	fn has_legal(&self) -> bool {
		self.license_year.is_some() || self.license_name.is_some() || self.license_text.is_some()
	}

	fn details(&self, ui: &mut egui::Ui) {
		if let Some(developer) = &self.developer {
			ui.label(format!("Developed by {developer}"));
		}

		if self.developer.is_some() && (self.website_url.is_some() || self.issues_url.is_some()) {
			ui.add_space(6.0);
		}

		if let Some(website_url) = &self.website_url {
			ui.hyperlink_to("Website", website_url);
		}
		if let Some(issues_url) = &self.issues_url {
			ui.hyperlink_to("Report an issue", issues_url);
		}

		if let Some(description_long) = &self.description_long {
			if self.developer.is_some() || self.website_url.is_some() || self.issues_url.is_some() {
				ui.add_space(10.0);
			}
			ui.label(RichText::new(description_long).size(11.5).weak());
		}
	}

	fn legal(&self, ui: &mut egui::Ui) {
		if let Some(license_name) = &self.license_name {
			ui.label(RichText::new(license_name).strong());
		}

		if let Some(license_year) = self.license_year {
			if self.license_name.is_some() {
				ui.add_space(4.0);
			}
			let copyright = match &self.developer {
				Some(developer) => format!("© {license_year} {developer}"),
				None => format!("© {license_year}"),
			};
			ui.label(copyright);
		}

		if let Some(license_text) = &self.license_text {
			if self.license_name.is_some() || self.license_year.is_some() {
				ui.add_space(10.0);
			}
			ui.add(Label::new(RichText::new(license_text).text_style(TextStyle::Small)).wrap_mode(TextWrapMode::Wrap));
		}
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
				if let Some(icon_uri) = &self.icon_uri {
					ui.add(
						Image::from_uri(icon_uri)
							.fit_to_exact_size(egui::vec2(64.0, 64.0))
							.maintain_aspect_ratio(true),
					);
					ui.add_space(8.0);
				}
				ui.label(RichText::new(&self.name).size(20.0).strong());
				if let Some(description) = &self.description {
					ui.label(RichText::new(description).size(13.0).weak());
				}
				if let Some(version) = &self.version {
					if self.description.is_some() {
						ui.add_space(2.0);
					}
					ui.label(
						RichText::new(format!("Version {version}"))
							.size(12.0)
							.weak(),
					);
				}
				ui.add_space(12.0);
			});

			let has_details = self.has_details();
			let has_legal = self.has_legal();

			if has_details || has_legal {
				ui.separator();

				if has_details && has_legal {
					ui.horizontal(|ui| {
						ui.selectable_value(&mut self.tab, AboutTab::Details, "Details");
						ui.selectable_value(&mut self.tab, AboutTab::Legal, "Legal");
					});
					ui.separator();
				}

				ui.add_space(6.0);
				ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
					if has_details && (!has_legal || self.tab == AboutTab::Details) {
						self.details(ui);
					} else {
						self.legal(ui);
					}
				});
				ui.add_space(8.0);
			}

			ui.separator();
			ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
				if ui.button("Close").clicked() {
					self.open = false;
				}
			});
		});
	}
}
