use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
  pub language: String,
  pub checksum: u32,
  pub csv: String,
  pub font: String,
  pub encoding: String,
  pub code: Option<String>,
}

impl Default for Manifest {
  fn default() -> Self {
    Self {
      language: "-".to_string(),
      checksum: 0,
      csv: "".to_string(),
      font: "".to_string(),
      encoding: "".to_string(),
      code: None,
    }
  }
}

#[derive(Default)]
pub struct DictMetadata {
  pub manifest: Manifest,
  pub vec_manifests: Vec<Manifest>,
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

  pub fn pick_language_by_name(&mut self, language: String) {
    if let Some(manifest) = self
      .vec_manifests
      .iter()
      .find(|item| item.language == language)
    {
      self.manifest = manifest.clone();
    }
  }

  pub fn pick_language_by_code(&mut self, code: Option<String>) -> Option<String> {
    if let Some(manifest) = self.vec_manifests.iter().find(|item| item.code == code) {
      self.manifest = manifest.clone();
      return Some(manifest.language.clone());
    }
    None
  }
}
