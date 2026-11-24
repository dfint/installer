use anyhow::Result;
use std::path::PathBuf;

use crate::{
  app::{App, State},
  cache::Cache,
  constants::*,
  df_binary::DfBinary,
  dict_metadata::DictMetadata,
  fetch::batch_download_to_file,
  hook_metadata::HookMetadata,
  localization::{LOCALE, t},
  utils::*,
};

macro_rules! error {
  ($self:ident, $l:expr) => {
    $self.notice = Some($l);
  };
  ($self:ident, $l:expr, $e:expr) => {
    let message = $l;
    let _ = std::fs::write(
      PATH_ERROR_FILE,
      format!("{:?}\n{}\n{}", chrono::Local::now(), message, $e),
    );
    $self.notice = Some(message);
  };
}

pub enum Message {
  HookMetadataLoaded(Result<HookMetadata>),
  DictMetadataLoaded(Result<DictMetadata>),
  HookUpdated(Result<()>),
  DictUpdated(Result<()>),
  StartupDataLoaded((PathBuf, String, HookMetadata, DictMetadata)),
  DfRunning(bool),
}

impl App {
  /// Returns whether localization files or a modified font are present.
  pub fn any_localization_files_present(&self) -> bool {
    self.bin.lib_path.is_file()
      || self.bin.dir.join(PATH_CONFIG).is_file()
      || self.bin.dir.join(PATH_OFFSETS).is_file()
      || self.bin.dir.join(PATH_DICT).is_file()
      || self.bin.dir.join(PATH_ENCODING).is_file()
      || self.font_is_modified()
  }

  fn font_is_modified(&self) -> bool {
    std::fs::read(self.bin.dir.join(PATH_FONT)).is_ok_and(|font| crc32fast::hash(&font) != ORIGINAL_FONT_CHECKSUM)
  }

  pub fn select_binary(&mut self, file: PathBuf) {
    let bin = DfBinary::new(file);
    if !bin.valid {
      error!(self, t!("Selected file is not a valid Dwarf Fortress executable."));
      return;
    }
    self.bin = bin;
    self.notice = None;
    self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
    self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
    self.hook_metadata.pick_df_checksum(self.bin.checksum);
    self.delete_old_data_show = self.delete_old_data_check();
  }

  pub fn on_close(&mut self) {
    if self.bin.valid {
      self.settings.bin = self.bin.path.clone();
      self
        .settings
        .selected_language
        .clone_from(&self.selected_language);
    }
    if self.settings.save().is_ok() && self.bin.valid {
      let _ = Cache {
        hook_manifest: self.hook_metadata.manifest.clone(),
        vec_hook_manifests: self.hook_metadata.vec_manifests.clone(),
        dict_manifest: self.dict_metadata.manifest.clone(),
        vec_dict_manifests: self.dict_metadata.vec_manifests.clone(),
      }
      .save();
    }
  }

  pub fn update_state(&mut self) -> bool {
    let messages: Vec<_> = self.pool.poll().collect();
    let changed = !messages.is_empty();
    for msg in messages {
      match msg {
        Message::HookMetadataLoaded(result) => {
          self.hook_metadata_loading = false;
          match result {
            Ok(metadata) => {
              self.hook_metadata = metadata;
              self.hook_metadata.pick_df_checksum(self.bin.checksum);
              if self.hook_metadata.manifest.checksum == 0 && self.bin.valid {
                error!(self, t!("This DF version is not supported"));
              }
            }
            Err(err) => {
              error!(self, t!("Unable to fetch hook metadata..."), err.to_string());
            }
          }
        }
        Message::DictMetadataLoaded(result) => {
          self.dict_metadata_loading = false;
          match result {
            Ok(metadata) => {
              self.dict_metadata = metadata;
              self
                .dict_metadata
                .pick_language_by_name(self.selected_language.clone());
              if self.selected_language == "None"
                && let Some(language) = self.dict_metadata.pick_language_by_code(Some(
                  sys_locale::get_locale()
                    .unwrap_or("en-US".to_string())
                    .split('-')
                    .collect::<Vec<&str>>()[0]
                    .to_owned(),
                ))
              {
                self.selected_language = language
              }
            }
            Err(err) => {
              error!(self, t!("Unable to fetch hook metadata..."), err.to_string());
            }
          }
        }
        Message::HookUpdated(result) => {
          match result {
            Ok(_) => {
              self.notice = Some(t!("Hook updated"));
              self.hook_checksum = self.local_hook_checksum().unwrap_or(0)
            }
            Err(err) => {
              error!(self, t!("Unable to update hook..."), err.to_string());
            }
          };
          self.loading -= 1;
        }
        Message::DictUpdated(result) => {
          match result {
            Ok(_) => {
              self.notice = Some(t!("Dictionary updated"));
              self.dict_checksum = self.local_dict_checksum().unwrap_or(0)
            }
            Err(err) => {
              let is_permission_denied = err.chain().any(|cause| {
                if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
                  io_err.kind() == std::io::ErrorKind::PermissionDenied
                } else {
                  false
                }
              });

              error!(
                self,
                match is_permission_denied {
                  true => t!("Permission denied, check if the directory and files are writable"),
                  false => t!("Unable to update dictionary"),
                },
                err.to_string()
              );
            }
          };
          self.loading -= 1;
        }
        Message::StartupDataLoaded((bin, selected_language, hook_metadata, dict_metadata)) => {
          self.bin = DfBinary::new(bin);
          self.selected_language = selected_language;
          self.hook_metadata = hook_metadata;
          self.dict_metadata = dict_metadata;

          self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
          self.hook_metadata_loading = true;
          let checksum = self.bin.checksum;
          self.pool.execute(
            move || HookMetadata::from_url(URL_HOOK_MANIFEST, Some(checksum)),
            Message::HookMetadataLoaded,
          );

          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          self.dict_metadata_loading = true;
          let selected_language = self.selected_language.clone();
          self.pool.execute(
            move || DictMetadata::from_url(URL_DICT_MANIFEST, Some(selected_language)),
            Message::DictMetadataLoaded,
          );

          if self.selected_language == "None"
            && let Some(language) = self.dict_metadata.pick_language_by_code(Some(
              sys_locale::get_locale()
                .unwrap_or("en-US".to_string())
                .split('-')
                .collect::<Vec<&str>>()[0]
                .to_owned(),
            ))
          {
            self.selected_language = language;
          }

          self.delete_old_data_show = self.delete_old_data_check();
          self.state = State::Idle;
        }
        Message::DfRunning(result) => {
          self.df_running = result;
          self.process_checked = true;
        }
      }
    }
    changed
  }

  pub fn on_start(&mut self) {
    self.state = State::Loading;
    self.pool.execute(is_df_running, Message::DfRunning);
    let saved_bin = self.settings.bin.clone();
    let selected_language = self.settings.selected_language.clone();
    self.pool.execute(
      move || {
        let bin = if saved_bin.exists() {
          saved_bin
        } else {
          scan_df().unwrap_or_else(|| std::env::current_dir().unwrap())
        };
        let (hook_metadata, dict_metadata) = Cache::load_metadata();
        (bin, selected_language, hook_metadata, dict_metadata)
      },
      Message::StartupDataLoaded,
    );
  }

  pub fn delete_old_data_check(&self) -> bool {
    if !self.bin.valid {
      return false;
    }
    self.bin.dir.join("dfint_launcher.exe").exists() || self.bin.dir.join("dfint_data").exists()
  }

  pub fn confirm_old_data_removal(&mut self) {
    self.remove_old_data();
    self.notice = Some(t!("Old files successfully deleted"));
  }

  pub fn confirm_localization_removal(&mut self) {
    self.remove_hook_data();
    self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
    self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
    self.notice = Some(t!("Localization files successfully deleted"));
  }

  pub fn update_data(&mut self) {
    if !self.ready() || !self.bin.valid || self.loading > 0 {
      return;
    }
    if let Err(err) = std::fs::create_dir_all(self.bin.dir.join(PATH_DATA)) {
      error!(self, t!("Unable to create directory"), err.to_string());
      return;
    }

    if self.hook_update_available() {
      let hook_manifest = self.hook_metadata.manifest.clone();
      self.loading += 1;
      let downloads = vec![
        (hook_manifest.lib, self.bin.lib_path.clone()),
        (hook_manifest.config, self.bin.dir.join(PATH_CONFIG)),
        (hook_manifest.offsets, self.bin.dir.join(PATH_OFFSETS)),
        (hook_manifest.dfhooks, self.bin.dfhooks_path.clone()),
      ];
      self
        .pool
        .execute(move || batch_download_to_file(downloads), Message::HookUpdated);
    }

    if self.dictionary_update_available() {
      let dict_manifest = self.dict_metadata.manifest.clone();
      self.loading += 1;
      let downloads = vec![
        (dict_manifest.csv, self.bin.dir.join(PATH_DICT)),
        (dict_manifest.font, self.bin.dir.join(PATH_FONT)),
        (dict_manifest.encoding, self.bin.dir.join(PATH_ENCODING)),
      ];
      self
        .pool
        .execute(move || batch_download_to_file(downloads), Message::DictUpdated);
    }
  }

  pub fn local_hook_checksum(&self) -> Result<u32> {
    checksum_for_files(vec![
      self.bin.lib_path.clone(),
      self.bin.dir.join(PATH_CONFIG),
      self.bin.dir.join(PATH_OFFSETS),
      self.bin.dfhooks_path.clone(),
    ])
  }

  pub fn local_dict_checksum(&self) -> Result<u32> {
    checksum_for_files(vec![
      self.bin.dir.join(PATH_DICT),
      self.bin.dir.join(PATH_FONT),
      self.bin.dir.join(PATH_ENCODING),
    ])
  }

  pub fn remove_old_data(&self) {
    let _ = std::fs::remove_file(self.bin.dir.join("dfint_launcher.exe"));
    let _ = std::fs::remove_dir_all(self.bin.dir.join("dfint_data"));
  }

  pub fn remove_hook_data(&self) {
    let _ = std::fs::write(self.bin.dir.join(PATH_FONT), ORIGINAL_FONT);
    let _ = std::fs::remove_file(self.bin.lib_path.clone());
    let _ = std::fs::remove_dir_all(self.bin.dir.join("dfint-data"));
  }
}
