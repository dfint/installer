use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
  pub language: String,
  pub checksum: u32,
  pub csv: String,
  pub font: String,
  pub encoding: String,
}

impl Default for Manifest {
  fn default() -> Self {
    Self {
      language: "-".to_string(),
      checksum: 0,
      csv: "".to_string(),
      font: "".to_string(),
      encoding: "".to_string(),
    }
  }
}

pub struct DictMetadata {
  pub manifest: Manifest,
  pub vec_manifests: Vec<Manifest>,
}

impl Default for DictMetadata {
  fn default() -> Self {
    Self {
      manifest: Manifest::default(),
      vec_manifests: vec![],
    }
  }
}

impl DictMetadata {
  pub async fn from_url(url: &str, pick_language: Option<String>) -> Result<Self> {
    let manifests: Vec<Manifest> = ureq::get(url).call()?.into_json()?;

    let picked = match pick_language {
      Some(language) => {
        if let Some(manifest) = manifests.iter().find(|item| item.language == language) {
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

  pub fn pick_language(&mut self, language: String) {
    if let Some(manifest) = self
      .vec_manifests
      .iter()
      .find(|item| item.language == language)
    {
      self.manifest = manifest.clone();
    }
  }
}
