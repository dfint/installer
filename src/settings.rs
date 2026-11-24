use anyhow::Result;
use std::path::PathBuf;

use crate::constants::{PATH_CACHE_FILE, PATH_SETTINGS_FILE};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
  pub bin: PathBuf,
  pub selected_language: String,
  pub theme: String,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      bin: PathBuf::new(),
      selected_language: "None".into(),
      theme: "system".into(),
    }
  }
}

impl Settings {
  pub fn load() -> Self {
    let content = match std::fs::read_to_string(PATH_SETTINGS_FILE) {
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => std::fs::read_to_string(PATH_CACHE_FILE),
      result => result,
    };
    content
      .ok()
      .and_then(|content| serde_json::from_str(&content).ok())
      .unwrap_or_default()
  }

  pub fn save(&self) -> Result<()> {
    std::fs::write(PATH_SETTINGS_FILE, serde_json::to_string_pretty(self)?)?;
    Ok(())
  }
}
