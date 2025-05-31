use anyhow::Result;
use std::path::PathBuf;

use crate::{
  constants::PATH_CACHE_FILE,
  dict_metadata::{DictMetadata, Manifest as DictManifest},
  hook_metadata::{HookMetadata, Manifest as HookManifest},
  utils::scan_df,
};

#[derive(Serialize, Deserialize)]
pub struct Store {
  pub bin: String,
  pub hook_manifest: HookManifest,
  pub vec_hook_manifests: Vec<HookManifest>,
  pub dict_manifest: DictManifest,
  pub vec_dict_manifests: Vec<DictManifest>,
  pub selected_language: String,
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
