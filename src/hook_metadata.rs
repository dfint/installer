use anyhow::Result;

use crate::fetch;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Manifest {
  pub df: u32,
  pub checksum: u32,
  pub lib: String,
  pub config: String,
  pub offsets: String,
  pub dfhooks: String,
}

#[derive(Default)]
pub struct HookMetadata {
  pub manifest: Manifest,
  pub vec_manifests: Vec<Manifest>,
}

impl HookMetadata {
  pub async fn from_url(url: &str, pick_df_checksum: Option<u32>) -> Result<Self> {
    let manifests: Vec<Manifest> = fetch::fetch_json(url)?;

    let picked = match pick_df_checksum {
      Some(checksum) => {
        if let Some(manifest) = manifests.iter().find(|item| item.df == checksum) {
          manifest.clone()
        } else {
          Manifest::default()
        }
      }
      None => Manifest::default(),
    };

    Ok(Self {
      manifest: picked,
      vec_manifests: manifests,
    })
  }

  pub fn pick_df_checksum(&mut self, checksum: u32) {
    if let Some(manifest) = self.vec_manifests.iter().find(|item| item.df == checksum) {
      self.manifest = manifest.clone();
    } else {
      self.manifest = Manifest::default();
    }
  }
}
