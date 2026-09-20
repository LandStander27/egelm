use std::{os::unix::ffi::OsStringExt, path::PathBuf};

use crate::error::Error;

use super::PortalRequest;

use ashpd::desktop::file_chooser::FileFilter;

fn uri_to_path(uri: &str) -> Option<PathBuf> {
	let path_str = uri.strip_prefix("file://")?;

	let mut bytes = Vec::with_capacity(path_str.len());
	let mut chars = path_str.as_bytes().iter().copied();
	while let Some(b) = chars.next() {
		if b == b'%' {
			let h1 = chars.next()?;
			let h2 = chars.next()?;
			let v = [h1, h2];
			let hex_str = std::str::from_utf8(&v).ok()?;
			let byte = u8::from_str_radix(hex_str, 16).ok()?;
			bytes.push(byte);
		} else {
			bytes.push(b);
		}
	}

	Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

/// Request to open one or more files or folders via the desktop portal.
#[derive(Debug, Clone, Default)]
pub struct OpenFile {
	title: Option<String>,
	accept_label: Option<String>,
	multiple: bool,
	dir: bool,
	filters: Vec<FileFilter>,
}

impl OpenFile {
	/// Creates a default open-file dialog request.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the window title of the file chooser dialog.
	pub fn title(mut self, title: impl Into<String>) -> Self {
		self.title = Some(title.into());
		self
	}

	/// Sets the label for the accept button (e.g. "Select", "Open").
	pub fn accept_label(mut self, label: impl Into<String>) -> Self {
		self.accept_label = Some(label.into());
		self
	}

	/// Allows the user to select multiple files.
	pub fn multiple(mut self, multiple: bool) -> Self {
		self.multiple = multiple;
		self
	}

	/// Allows the user to select folders instead of files.
	pub fn directory(mut self, directory: bool) -> Self {
		self.dir = directory;
		self
	}

	/// Adds a filter for selectable files.
	pub fn filter(mut self, filter: FileFilter) -> Self {
		self.filters.push(filter);
		self
	}
}

#[async_trait::async_trait]
impl PortalRequest for OpenFile {
	type Output = Vec<PathBuf>;

	async fn send(self, window: Option<ashpd::WindowIdentifier>) -> Result<Self::Output, Error> {
		Ok(ashpd::desktop::file_chooser::OpenFileRequest::default()
			.multiple(self.multiple)
			.directory(self.dir)
			.title(self.title.as_deref())
			.accept_label(self.accept_label.as_deref())
			.filters(self.filters)
			.identifier(window)
			.modal(true)
			.send()
			.await
			.map_err(Error::Portal)?
			.response()
			.map_err(Error::Portal)?
			.uris()
			.iter()
			.filter_map(|uri| uri_to_path(uri.as_str()))
			.collect()) // holy chain
	}
}

/// Request to save a file via the desktop portal.
#[derive(Debug, Clone, Default)]
pub struct SaveFile {
	title: Option<String>,
	accept_label: Option<String>,
	current_name: Option<String>,
	filters: Vec<FileFilter>,
}

impl SaveFile {
	/// Creates a default save-file dialog request.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the window title of the save dialog.
	pub fn title(mut self, title: impl Into<String>) -> Self {
		self.title = Some(title.into());
		self
	}

	/// Sets the label for the accept button (e.g. "Save").
	pub fn accept_label(mut self, label: impl Into<String>) -> Self {
		self.accept_label = Some(label.into());
		self
	}

	/// Sets the suggested file name for saving.
	pub fn current_name(mut self, name: impl Into<String>) -> Self {
		self.current_name = Some(name.into());
		self
	}

	/// Adds a filter for selectable file types.
	pub fn filter(mut self, filter: FileFilter) -> Self {
		self.filters.push(filter);
		self
	}
}

#[async_trait::async_trait]
impl PortalRequest for SaveFile {
	type Output = Vec<PathBuf>;

	async fn send(self, window: Option<ashpd::WindowIdentifier>) -> Result<Self::Output, Error> {
		Ok(ashpd::desktop::file_chooser::SaveFileRequest::default()
			.title(self.title.as_deref())
			.accept_label(self.accept_label.as_deref())
			.current_name(self.current_name.as_deref())
			.filters(self.filters)
			.identifier(window)
			.send()
			.await
			.map_err(Error::Portal)?
			.response()
			.map_err(Error::Portal)?
			.uris()
			.iter()
			.filter_map(|uri| uri_to_path(uri.as_str()))
			.collect())
	}
}
