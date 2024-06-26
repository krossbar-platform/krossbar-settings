use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    path::Path,
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Settings handle
pub struct Settings {
    /// Settings file handle
    settings_file: File,
}

impl Settings {
    /// Open settings file at **path**
    pub fn open(path: &Path) -> crate::Result<Self> {
        // No settings fiel. Let's create and init one
        let settings_file = if !Path::new(path).exists() {
            let mut file =
                File::create_new(path).map_err(|e| crate::Error::IoError(e.to_string()))?;

            file.write_all("{}".as_bytes())
                .map_err(|e| crate::Error::IoError(e.to_string()))?;

            file
        // Existing settings file
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(|e| crate::Error::IoError(e.to_string()))?
        };

        Ok(Self { settings_file })
    }

    /// Read a value from the settings file
    pub fn get<T: DeserializeOwned>(&mut self, key: &str) -> crate::Result<T> {
        self.modify_settings(false, |map| {
            if let Some(settings_value) = map.remove(key) {
                serde_json::from_value(settings_value)
                    .map_err(|e| crate::Error::Type(e.to_string()))
            } else {
                Err(crate::Error::NotFound)
            }
        })
    }

    /// Check if there's a value with a given **key**
    pub fn has_value(&mut self, key: &str) -> crate::Result<bool> {
        self.modify_settings(false, |map| Ok(map.contains_key(key)))
    }

    /// Write new value in the settings file
    pub fn set<T: Serialize>(&mut self, key: &str, value: &T) -> crate::Result<()> {
        self.modify_settings(true, |map| {
            let json_value =
                serde_json::to_value(value).map_err(|e| crate::Error::Type(e.to_string()))?;

            map.insert(key.to_owned(), json_value);
            Ok(())
        })
    }

    /// Clear out entry with a given **key** from the file
    pub fn clear(&mut self, key: &str) -> crate::Result<()> {
        self.modify_settings(true, |map| {
            map.remove(key);
            Ok(())
        })
    }

    /// List value in the settings file
    pub fn list_values(&mut self) -> crate::Result<Vec<(String, Value)>> {
        self.modify_settings(false, |map| {
            let keys: Vec<String> = map.keys().cloned().collect();

            Ok(keys
                .into_iter()
                .map(|key| {
                    let value = map.remove(&key).unwrap();
                    (key, value)
                })
                .collect())
        })
    }

    fn modify_settings<T>(
        &mut self,
        write_back: bool,
        func: impl Fn(&mut serde_json::Map<String, Value>) -> crate::Result<T>,
    ) -> crate::Result<T> {
        // Start reading from the beginning
        self.settings_file
            .seek(std::io::SeekFrom::Start(0))
            .map_err(|e| crate::Error::IoError(e.to_string()))?;

        let mut data = Vec::new();
        // Read settings JSON data
        self.settings_file
            .read_to_end(&mut data)
            .map_err(|e| crate::Error::IoError(e.to_string()))?;

        // Convert to JSON
        let json: Value =
            serde_json::from_slice(&data).map_err(|e| crate::Error::Corrupted(e.to_string()))?;

        if let Value::Object(mut map) = json {
            let result = func(&mut map);

            // If write back
            if write_back && result.is_ok() {
                // Start writing from the beggining of the file
                self.settings_file
                    .seek(std::io::SeekFrom::Start(0))
                    .map_err(|e| crate::Error::IoError(e.to_string()))?;

                // Truncate all the content
                self.settings_file
                    .set_len(0)
                    .map_err(|e| crate::Error::IoError(e.to_string()))?;

                // JSON to data
                let data_to_write = serde_json::to_vec_pretty(&Value::Object(map))
                    .map_err(|e| crate::Error::Type(e.to_string()))?;

                // Write JSON
                self.settings_file
                    .write_all(&data_to_write)
                    .map_err(|e| crate::Error::IoError(e.to_string()))?;
            }

            result
        } else {
            Err(crate::Error::Corrupted(
                "Root settings elemetn is not an Object".into(),
            ))
        }
    }
}
