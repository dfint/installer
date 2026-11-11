use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;

use crate::{
  app::{App, State},
  constants::*,
  df_binary::DfBinary,
  dict_metadata::DictMetadata,
  fetch::batch_download_to_file,
  hook_metadata::HookMetadata,
  localization::{LOCALE, t},
  persistent::Store,
  utils::*,
};

macro_rules! error {
  ($self:ident, $l:expr) => {
    $self.toast.error($l);
  };
  ($self:ident, $l:expr, $e:expr) => {
    $self.toast.error($l);
    std::fs::write(PATH_ERROR_FILE, format!("{:?}\n{}\n{}", chrono::Local::now(), $l, $e)).unwrap();
  };
}

pub enum Message {
  HookMetadataLoaded(Result<HookMetadata>),
  DictMetadataLoaded(Result<DictMetadata>),
  HookUpdated(Result<()>),
  DictUpdated(Result<()>),
  StoreLoaded((PathBuf, String, HookMetadata, DictMetadata)),
  DfRunning(bool),
}

impl App {
  /// Returns whether any localization data is installed.
  pub fn translation_installed(&self) -> bool {
    self.hook_checksum != NO_HOOK_CHECKSUM || self.dict_checksum != NO_DICT_CHECKSUM
  }

  /// Shows a native file picker filtered to Dwarf Fortress executables.
  ///
  /// Starts in `dir`, or the current working directory when `dir` is `None`.
  /// Returns the selected executable path, or `None` if the user cancels the dialog.
  pub fn file_dialog(&self, dir: Option<PathBuf>) -> Option<PathBuf> {
    let start_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    rfd::FileDialog::new()
      .set_title(&t!("Open Dwarf Fortress executable"))
      .set_directory(start_dir)
      .add_filter("Executable", &["exe", "dwarfort"])
      .pick_file()
  }

  /// Handles the file-selection flow for the native Dwarf Fortress picker.
  ///
  /// When a valid executable is chosen, the app updates the binary state and related
  /// metadata. If the user cancels the dialog and no valid binary is available, the
  /// window is closed to avoid leaving the installer in an unusable state.
  pub fn opened_file_dialog(&mut self, ctx: &egui::Context) {
    if !self.file_picker_requested {
      return;
    }

    self.file_picker_requested = false;

    if let Some(file) = self.file_dialog(Some(self.bin.dir.clone())) {
      self.bin = DfBinary::new(file);
      self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
      self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
      self.hook_metadata.pick_df_checksum(self.bin.checksum);
      self.delete_hook_show = self.delete_old_data_check();
    } else if !self.bin.valid {
      self.closing = true;
      ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
  }

  pub fn on_close(&mut self) {
    if self.bin.valid {
      let _ = Store {
        bin: self.bin.to_string(),
        hook_manifest: self.hook_metadata.manifest.clone(),
        vec_hook_manifests: self.hook_metadata.vec_manifests.clone(),
        dict_manifest: self.dict_metadata.manifest.clone(),
        vec_dict_manifests: self.dict_metadata.vec_manifests.clone(),
        selected_language: self.selected_language.clone(),
      }
      .save();
    }
  }

  pub fn update_state(&mut self) {
    for msg in self.pool.poll() {
      match msg {
        Message::HookMetadataLoaded(result) => {
          self.hook_metadata_loading = false;
          match result {
            Ok(metadata) => {
              self.hook_metadata = metadata;
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
              self.toast.success(t!("Hook updated"));
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
              self.toast.success(t!("Dictionary updated"));
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
        Message::StoreLoaded((bin, selected_language, hook_metadata, dict_metadata)) => {
          self.bin = DfBinary::new(bin);
          self.selected_language = selected_language;
          self.hook_metadata = hook_metadata;
          self.dict_metadata = dict_metadata;

          self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
          self.hook_metadata_loading = true;
          self.pool.execute(
            HookMetadata::from_url(URL_HOOK_MANIFEST, Some(self.bin.checksum)),
            Message::HookMetadataLoaded,
          );

          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          self.dict_metadata_loading = true;
          self.pool.execute(
            DictMetadata::from_url(URL_DICT_MANIFEST, Some(self.selected_language.clone())),
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
        }
      }
    }
  }

  pub fn on_start(&mut self) {
    self.state = State::Loading;
    self.pool.execute(is_df_running(), Message::DfRunning);
    self.pool.execute(Store::new(), Message::StoreLoaded);
  }

  /// Shows a warning identified by `name` and requests window closure when OK is clicked.
  pub fn guard(&mut self, ctx: &egui::Context, name: &str, text: &str) {
    egui::Modal::new(egui::Id::unique(name)).show(ctx, |ui| {
      ui.heading(t!("Warning"));
      ui.add_space(8.);
      ui.label(text);
      ui.add_space(8.);
      ui.vertical_centered(|ui| {
        if ui.button(t!("Ok")).clicked() {
          ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
      });
    });
  }

  pub fn delete_old_data_check(&self) -> bool {
    if !self.bin.valid {
      return false;
    }
    self.bin.dir.join("dfint_launcher.exe").exists() || self.bin.dir.join("dfint_data").exists()
  }

  pub fn delete_old_hook_dialog(&mut self, ctx: &egui::Context) {
    self.dialog(
      ctx,
      "delete_old_data",
      t!("Old version of translation files has been detected. It's better to delete them to avoid conflicts. Delete?"),
      |app| {
        app.delete_old_data_show = false;
      },
      |app| {
        app.delete_old_data_show = false;
        app.remove_old_data();
        app.toast.success(t!("Old files successfully deleted"));
      },
    );
  }

  pub fn delete_hook_dialog(&mut self, ctx: &egui::Context) {
    self.dialog(
      ctx,
      "delete_data",
      t!("Delete all localization files?"),
      |app| {
        app.delete_hook_show = false;
        app.selected_language = "English".to_string();
      },
      |app| {
        app.delete_hook_show = false;
        app.remove_hook_data();
        app.hook_checksum = app.local_hook_checksum().unwrap_or(0);
        app.dict_checksum = app.local_dict_checksum().unwrap_or(0);
        app
          .toast
          .success(t!("Localization files successfully deleted"));
      },
    );
  }

  /// Shows a confirmation modal identified by `tag` with the supplied warning text.
  ///
  /// Invokes `no` or `yes` when the corresponding button is clicked. Callbacks must
  /// update application state to stop showing the dialog on subsequent frames.
  fn dialog(
    &mut self,
    ctx: &egui::Context,
    tag: impl std::fmt::Display,
    text: String,
    no: impl FnOnce(&mut App),
    yes: impl FnOnce(&mut App),
  ) {
    egui::Modal::new(egui::Id::unique(tag.to_string())).show(ctx, |ui| {
      ui.heading(t!("Warning"));
      ui.add_space(8.);
      ui.label(text);
      ui.add_space(8.);
      ui.horizontal(|ui| {
        if ui.button(t!("No")).clicked() {
          no(self);
        }
        if ui.button(t!("Yes")).clicked() {
          yes(self);
        }
      });
    });
  }

  pub fn update_data(&mut self) {
    std::fs::create_dir_all(self.bin.dir.join(PATH_DATA)).expect("Unable to create directory");

    let hook_manifest = self.hook_metadata.manifest.clone();
    if hook_manifest.df == self.bin.checksum && hook_manifest.checksum != self.hook_checksum {
      self.loading += 1;
      self.pool.execute(
        batch_download_to_file(vec![
          (hook_manifest.lib, self.bin.lib_path.clone()),
          (hook_manifest.config, self.bin.dir.join(PATH_CONFIG)),
          (hook_manifest.offsets, self.bin.dir.join(PATH_OFFSETS)),
          (hook_manifest.dfhooks, self.bin.dfhooks_path.clone()),
        ]),
        Message::HookUpdated,
      );
    }

    let dict_manifest = self.dict_metadata.manifest.clone();
    if dict_manifest.checksum != self.dict_checksum && self.selected_language != "None" {
      self.loading += 1;
      self.pool.execute(
        batch_download_to_file(vec![
          (dict_manifest.csv, self.bin.dir.join(PATH_DICT)),
          (dict_manifest.font, self.bin.dir.join(PATH_FONT)),
          (dict_manifest.encoding, self.bin.dir.join(PATH_ENCODING)),
        ]),
        Message::DictUpdated,
      );
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
