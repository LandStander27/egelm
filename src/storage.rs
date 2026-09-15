use std::sync::Arc;

use crate::prelude::*;

pub(crate) mod file;
pub(crate) mod memory;

/// Abstraction for persisting key-value configuration data.
pub trait StorageBackend: std::fmt::Debug + Send + Sync {
	/// Retrieves raw bytes stored under the given key.
	fn get(&self, name: &str) -> Result<Option<Vec<u8>>, Error>;
	/// Stores raw bytes under the given key.
	fn set(&self, name: &str, data: Vec<u8>) -> Result<(), Error>;
	/// Removes the stored value under the given key.
	fn remove(&self, name: &str) -> Result<(), Error>;
	/// Clears all data.
	fn clear(&self) -> Result<(), Error>;
	/// Flushes all pending changes to the persistent storage immediately.
	fn flush(&self) -> Result<(), Error>;
}

/// A handle to the application’s persistent configuration storage.
#[derive(Debug, Clone)]
pub struct Storage {
	backend: Arc<dyn StorageBackend>,
}

impl Default for Storage {
	fn default() -> Self {
		Self {
			backend: Arc::new(memory::MemoryBackend::default()),
		}
	}
}

impl Storage {
	pub(crate) fn new<T: StorageBackend + 'static>(backend: T) -> Self {
		Self { backend: Arc::new(backend) }
	}

	/// Retrieves and deserializes a value from storage under the given key.
	pub fn get<T: serde::de::DeserializeOwned>(&self, name: impl AsRef<str>) -> Result<Option<T>, Error> {
		let Some(raw) = self.backend.get(name.as_ref())? else { return Ok(None) };
		Ok(Some(ron::de::from_bytes(&raw).map_err(Error::StorageParsing)?))
	}

	/// Serializes and stores a value under the given key.
	pub fn set<T: serde::ser::Serialize>(&self, name: impl AsRef<str>, value: &T) -> Result<(), Error> {
		let raw = ron::ser::to_string(value).map_err(Error::StorageSerialization)?;
		self.backend.set(name.as_ref(), raw.into_bytes())
	}

	/// Removes the stored value under the given key.
	pub fn remove(&self, name: impl AsRef<str>) -> Result<(), Error> {
		self.backend.remove(name.as_ref())
	}

	/// Manually flushes all pending storage operations to their persistent backend.
	pub fn flush(&self) -> Result<(), Error> {
		self.backend.flush()
	}

	/// Clears all data.
	pub fn clear(&self) -> Result<(), Error> {
		self.backend.clear()
	}
}

impl Drop for Storage {
	fn drop(&mut self) {
		if let Err(e) = self.flush() {
			tracing::error!("failed to flush storage on drop: {e}");
		}
	}
}
