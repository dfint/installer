use anyhow::Result;
use egui::ThemePreference;
use std::path::PathBuf;

use crate::{
  constants::PATH_CACHE_FILE,
  dict_metadata::{DictMetadata, Manifest as DictManifest},
  hook_metadata::{HookMetadata, Manifest as HookManifest},
  utils::scan_df,
};

pub fn load_theme() -> ThemePreference {
  let cache = read_cache().unwrap_or_default();
  match cache.get("theme").and_then(serde_json::Value::as_str) {
    Some("light") => ThemePreference::Light,
    Some("dark") => ThemePreference::Dark,
    _ => ThemePreference::System,
  }
}

fn read_cache() -> Result<serde_json::Map<String, serde_json::Value>> {
  match std::fs::read_to_string(PATH_CACHE_FILE) {
    Ok(content) => Ok(serde_json::from_str(&content)?),
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
    Err(err) => Err(err.into()),
  }
}

pub fn save_theme(theme: ThemePreference) -> Result<()> {
  let mut cache = read_cache()?;
  cache.insert("theme".into(), theme.name().into());
  std::fs::write(PATH_CACHE_FILE, serde_json::to_string_pretty(&cache)?)?;
  Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct Store {
  pub bin: String,
  pub hook_manifest: HookManifest,
  pub vec_hook_manifests: Vec<HookManifest>,
  pub dict_manifest: DictManifest,
  pub vec_dict_manifests: Vec<DictManifest>,
  pub selected_language: String,
  #[serde(default)]
  pub theme: String,
}

impl Store {
  fn load() -> Result<Self> {
    let content = std::fs::read_to_string(PATH_CACHE_FILE)?;
    let store: Store = serde_json::from_str(&content)?;
    Ok(store)
  }

  pub fn save(&self) -> Result<()> {
    std::fs::write(PATH_CACHE_FILE, serde_json::to_string_pretty(self)?)?;
    Ok(())
  }

  pub async fn new() -> (PathBuf, String, HookMetadata, DictMetadata) {
    match Store::load() {
      Ok(store) => {
        let mut bin = PathBuf::from(store.bin);
        if !bin.exists() {
          bin = scan_df().unwrap_or(std::env::current_dir().unwrap().to_path_buf());
        }
        (
          bin,
          store.selected_language,
          HookMetadata {
            manifest: store.hook_manifest,
            vec_manifests: store.vec_hook_manifests,
          },
          DictMetadata {
            manifest: store.dict_manifest,
            vec_manifests: store.vec_dict_manifests,
          },
        )
      }
      Err(_) => (
        scan_df().unwrap_or(std::env::current_dir().unwrap().to_path_buf()),
        String::from("None"),
        HookMetadata::default(),
        DictMetadata::default(),
      ),
    }
  }
}
