use anyhow::Result;

use crate::constants::PATH_CACHE_FILE;
use crate::utils::{DictManifest, HookManifest};

#[derive(Serialize, Deserialize)]
pub struct Store {
  pub df_bin: String,
  pub hook_manifest: HookManifest,
  pub vec_hook_manifests: Vec<HookManifest>,
  pub dict_manifest: DictManifest,
  pub vec_dict_manifests: Vec<DictManifest>,
  pub selected_language: String,
}

pub fn save(store: Store) -> Result<()> {
  let _ = std::fs::write(PATH_CACHE_FILE, serde_json::to_string_pretty(&store)?)?;
  Ok(())
}

pub fn load() -> Result<Store> {
  let content = std::fs::read_to_string(PATH_CACHE_FILE)?;
  let store: Store = serde_json::from_str(&content)?;
  Ok(store)
}
