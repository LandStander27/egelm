use std::collections::HashMap;

use crate::prelude::*;

use super::StorageBackend;

#[derive(Default, Debug)]
pub struct MemoryBackend {
	values: std::sync::RwLock<HashMap<String, Vec<u8>>>,
}

impl StorageBackend for MemoryBackend {
	fn get(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
		let values = self.values.read().unwrap();
		Ok(values.get(name).cloned())
	}

	fn set(&self, name: &str, data: Vec<u8>) -> Result<(), Error> {
		let mut values = self.values.write().unwrap();
		values.insert(name.to_string(), data);
		Ok(())
	}

	fn remove(&self, name: &str) -> Result<(), Error> {
		let mut values = self.values.write().unwrap();
		values.remove(name);
		Ok(())
	}

	fn clear(&self) -> Result<(), Error> {
		let mut values = self.values.write().unwrap();
		values.clear();
		Ok(())
	}

	fn flush(&self) -> Result<(), Error> {
		Ok(())
	}
}
