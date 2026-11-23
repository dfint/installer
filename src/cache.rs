use anyhow::Result;

use crate::{
  constants::PATH_CACHE_FILE,
  dict_metadata::{DictMetadata, Manifest as DictManifest},
  hook_metadata::{HookMetadata, Manifest as HookManifest},
};

#[derive(Default, Serialize, Deserialize)]
pub struct Cache {
  pub hook_manifest: HookManifest,
  pub vec_hook_manifests: Vec<HookManifest>,
  pub dict_manifest: DictManifest,
  pub vec_dict_manifests: Vec<DictManifest>,
}

impl Cache {
  fn load() -> Result<Self> {
    let content = std::fs::read_to_string(PATH_CACHE_FILE)?;
    Ok(serde_json::from_str(&content)?)
  }

  pub fn save(&self) -> Result<()> {
    std::fs::write(PATH_CACHE_FILE, serde_json::to_string_pretty(self)?)?;
    Ok(())
  }

  pub fn load_metadata() -> (HookMetadata, DictMetadata) {
    let cache = Self::load().unwrap_or_default();
    (
      HookMetadata {
        manifest: cache.hook_manifest,
        vec_manifests: cache.vec_hook_manifests,
      },
      DictMetadata {
        manifest: cache.dict_manifest,
        vec_manifests: cache.vec_dict_manifests,
      },
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::settings::Settings;
  use egui::ThemePreference;
  use std::path::PathBuf;

  #[test]
  fn legacy_cache_supplies_settings_and_manifests() {
    let mut legacy = serde_json::to_value(Cache::default()).unwrap();
    legacy["bin"] = serde_json::json!("dwarfort");
    legacy["selected_language"] = serde_json::json!("English");
    legacy["hook_manifest"]["checksum"] = serde_json::json!(42);

    let settings: Settings = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(settings.bin, PathBuf::from("dwarfort"));
    assert_eq!(settings.selected_language, "English");
    assert_eq!(settings.theme_preference(), ThemePreference::System);

    legacy["theme"] = serde_json::json!("dark");
    let settings: Settings = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(settings.theme_preference(), ThemePreference::Dark);

    let cache: Cache = serde_json::from_value(legacy).unwrap();
    assert_eq!(cache.hook_manifest.checksum, 42);
    let cache = serde_json::to_value(cache).unwrap();
    for key in ["bin", "selected_language", "theme"] {
      assert!(cache.get(key).is_none());
    }
  }
}
