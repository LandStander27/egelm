//! Opens files, directories, and URIs in default applications via the XDG desktop portal.

use std::fs::File;
use std::path::{Path, PathBuf};

use ashpd::Uri;
use ashpd::desktop::open_uri::{OpenDirectoryRequest, OpenFileRequest};

use super::PortalRequest;
use crate::error::Error;

#[derive(Debug, Clone)]
enum Target {
	Uri(String),
	File(PathBuf),
	Directory(PathBuf),
}

/// A portal request to open a file, directory, or URI in its default application.
#[derive(Debug, Clone)]
pub struct OpenURI {
	target: Target,
	writeable: bool,
	ask: bool,
}

impl OpenURI {
	/// Creates a request to open a web link or custom URI scheme (e.g. `https://example.com`, `mailto:someone@example.com`).
	pub fn uri(uri: impl Into<String>) -> Self {
		Self {
			target: Target::Uri(uri.into()),
			writeable: false,
			ask: false,
		}
	}

	/// Creates a request to open a local file in its default viewer/editor.
	pub fn file(path: impl AsRef<Path>) -> Self {
		Self {
			target: Target::File(path.as_ref().to_path_buf()),
			writeable: false,
			ask: false,
		}
	}

	/// Creates a request to open a directory in the default file manager.
	pub fn directory(path: impl AsRef<Path>) -> Self {
		Self {
			target: Target::Directory(path.as_ref().to_path_buf()),
			writeable: false,
			ask: false,
		}
	}

	/// Creates a request that automatically detects whether the input is a URI, directory, or file:
	/// - If it starts with a scheme like `http://` or `https://`, opens it as a URI.
	/// - If the path points to an existing directory on disk, opens it in the file manager.
	/// - Otherwise, opens it as a file.
	pub fn new(target: impl AsRef<str>) -> Self {
		let s = target.as_ref();
		if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("mailto:") {
			Self::uri(s)
		} else {
			let path = PathBuf::from(s.strip_prefix("file://").unwrap_or(s));
			if path.is_dir() {
				Self::directory(path)
			} else {
				Self::file(path)
			}
		}
	}

	/// Requests that the opened file be writeable by the target application.
	///
	/// Only applies to file targets.
	pub fn writeable(mut self, writeable: bool) -> Self {
		self.writeable = writeable;
		self
	}

	/// Asks the desktop environment to prompt the user for which application to use.
	pub fn ask(mut self, ask: bool) -> Self {
		self.ask = ask;
		self
	}
}

#[async_trait::async_trait]
impl PortalRequest for OpenURI {
	type Output = ();

	async fn send(self, window: Option<ashpd::WindowIdentifier>) -> Result<Self::Output, Error> {
		match self.target {
			Target::Uri(uri_str) => {
				let uri = Uri::parse(&uri_str)
					.map_err(ashpd::Error::from)
					.map_err(Error::Portal)?;
				OpenFileRequest::default()
					.identifier(window)
					.ask(self.ask)
					.send_uri(&uri)
					.await
					.map_err(Error::Portal)?
					.response()
					.map_err(Error::Portal)?;
			}
			Target::File(path) => {
				let file = File::open(&path).map_err(Error::ReadFailure)?;
				OpenFileRequest::default()
					.identifier(window)
					.writeable(self.writeable)
					.ask(self.ask)
					.send_file(&file)
					.await
					.map_err(Error::Portal)?
					.response()
					.map_err(Error::Portal)?;
			}
			Target::Directory(path) => {
				let file = File::open(&path).map_err(Error::ReadFailure)?;
				OpenDirectoryRequest::default()
					.identifier(window)
					.send(&file)
					.await
					.map_err(Error::Portal)?
					.response()
					.map_err(Error::Portal)?;
			}
		}

		Ok(())
	}
}
