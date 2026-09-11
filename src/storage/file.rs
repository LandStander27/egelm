use std::{collections::BTreeMap, path::PathBuf, sync::RwLock};

use crate::prelude::*;

use super::StorageBackend;

#[derive(Debug)]
pub(crate) struct FileBackend {
	file: PathBuf,
	lock: RwLock<StorageFile>,
}

impl FileBackend {
	pub(crate) fn new(file: PathBuf) -> Result<Self, Error> {
		let lock = RwLock::new(if file.exists() {
			let raw = std::fs::read(&file).map_err(Error::ReadFailure)?;
			ron::de::from_bytes(&raw).map_err(Error::StorageParsing)?
		} else {
			let data = StorageFile {
				version: 1,
				values: BTreeMap::new(),
			};
			let raw = ron::ser::to_string(&data).map_err(Error::StorageSerialization)?;
			std::fs::write(&file, raw).map_err(Error::WriteFailure)?;
			data
		});
		Ok(Self { file, lock })
	}
}

impl StorageBackend for FileBackend {
	fn get(&self, name: &str) -> Result<Option<Vec<u8>>, crate::prelude::Error> {
		let file = self.lock.read().unwrap();
		if let Some(value) = file.values.get(name) {
			Ok(Some(value.clone().into_bytes()))
		} else {
			Ok(None)
		}
	}

	fn set(&self, name: &str, data: Vec<u8>) -> Result<(), Error> {
		let mut file = self.lock.write().unwrap();
		file.values
			.insert(name.to_string(), String::from_utf8(data).map_err(Error::ExpectedUtf8)?);
		Ok(())
	}

	fn remove(&self, name: &str) -> Result<(), Error> {
		let mut file = self.lock.write().unwrap();
		file.values.remove(name);
		Ok(())
	}

	fn clear(&self) -> Result<(), Error> {
		let mut file = self.lock.write().unwrap();
		file.values.clear();
		Ok(())
	}

	fn flush(&self) -> Result<(), Error> {
		let file = self.lock.read().unwrap();
		let raw = ron::ser::to_string(&*file).map_err(Error::StorageSerialization)?;
		std::fs::write(&self.file, raw).map_err(Error::WriteFailure)?;
		Ok(())
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct StorageFile {
	version: u32,

	#[serde(default)]
	values: BTreeMap<String, String>,
}
