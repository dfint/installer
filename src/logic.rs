use anyhow::Result;
use eframe::egui;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::{
  app::{App, State},
  constants::*,
  df_binary::DfBinary,
  dict_metadata::DictMetadata,
  hook_metadata::HookMetadata,
  localization::{t, LOCALE},
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
  pub fn file_dialog(&self, dir: Option<PathBuf>) -> Option<egui_file::FileDialog> {
    let mut dialog = egui_file::FileDialog::open_file(self.opened_file.clone())
      .show_files_filter(Box::new(|path| {
        path.file_name() == Some(OsStr::new("Dwarf Fortress.exe")) || path.file_name() == Some(OsStr::new("dwarfort"))
      }))
      .resizable(false)
      .show_rename(false)
      .show_new_folder(false)
      .title(&t!("Open Dwarf Fortress executable"))
      .default_size([720., 381.]);
    dialog.set_path(dir.unwrap_or(std::env::current_dir().unwrap()));
    dialog.open();
    Some(dialog)
  }

  pub fn opened_file_dialog(&mut self, ctx: &egui::Context) {
    if let Some(dialog) = &mut self.open_file_dialog {
      if dialog.state() == egui_file::State::Closed && !self.bin.valid {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
      }
      if dialog.show(ctx).selected() {
        if let Some(file) = dialog.path() {
          self.bin = DfBinary::new(file.to_path_buf());
          self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          self.hook_metadata.pick_df_checksum(self.bin.checksum);
          self.delete_hook_show = self.delete_old_data_check();
        }
      }
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
        Message::HookMetadataLoaded(result) => match result {
          Ok(metadata) => {
            self.hook_metadata = metadata;
            if self.hook_metadata.manifest.checksum == 0 && self.bin.valid {
              error!(self, t!("This DF version is not supported"));
            }
          }
          Err(err) => {
            error!(self, t!("Unable to fetch hook metadata..."), err.to_string());
          }
        },
        Message::DictMetadataLoaded(result) => match result {
          Ok(metadata) => {
            self.dict_metadata = metadata;
          }
          Err(err) => {
            error!(self, t!("Unable to fetch hook metadata..."), err.to_string());
          }
        },
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
              error!(self, t!("Unable to update dictionary"), err.to_string());
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
          self.pool.execute(
            HookMetadata::from_url(URL_HOOK_MANIFEST, Some(self.bin.checksum)),
            Message::HookMetadataLoaded,
          );

          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          self.pool.execute(
            DictMetadata::from_url(URL_DICT_MANIFEST, Some(self.selected_language.clone())),
            Message::DictMetadataLoaded,
          );

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

  pub fn guard(&mut self, ctx: &egui::Context, name: &str, text: &str) {
    egui::CentralPanel::default().show(ctx, |_ui| {
      let modal = egui_modal::Modal::new(ctx, name);
      modal.show(|ui| {
        modal.title(ui, t!("Warning"));
        modal.frame(ui, |ui| {
          modal.body_and_icon(ui, text, egui_modal::Icon::Info);
        });
        modal.buttons(ui, |ui| {
          if modal.caution_button(ui, t!("Ok")).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
          }
        });
      });
      modal.open();
    });
  }

  pub fn delete_old_data_check(&self) -> bool {
    if !self.bin.valid {
      return false;
    }
    self.bin.dir.join("dfint_launcher.exe").exists() || self.bin.dir.join("dfint_data").exists()
  }

  pub fn delete_old_hook_dialog(&mut self, ctx: &egui::Context) {
    let modal = egui_modal::Modal::new(ctx, "delete_old_data");
    modal.show(|ui| {
      modal.title(ui, t!("Warning"));
      modal.frame(ui, |ui| {
        modal.body_and_icon(
          ui,
          t!("Old version of translation files has been detected. It's better to delete them to avoid conflicts. Delete?"),
          egui_modal::Icon::Info,
        );
      });
      modal.buttons(ui, |ui| {
        if modal.button(ui, t!("No")).clicked() {
          self.delete_old_data_show = false;
          modal.close();
        };
        if modal.suggested_button(ui, t!("Yes")).clicked() {
          self.delete_old_data_show = false;
          self.remove_old_data();
          modal.close();
          self.toast.success(t!("Old files successfully deleted"));
        };
      });
    });
    modal.open();
  }

  pub fn delete_hook_dialog(&mut self, ctx: &egui::Context) {
    let modal = egui_modal::Modal::new(ctx, "delete_data");
    modal.show(|ui| {
      modal.title(ui, t!("Warning"));
      modal.frame(ui, |ui| {
        modal.body_and_icon(ui, t!("Delete all localization files?"), egui_modal::Icon::Info);
      });
      modal.buttons(ui, |ui| {
        if modal.button(ui, t!("No")).clicked() {
          self.delete_hook_show = false;
          modal.close();
        };
        if modal.suggested_button(ui, t!("Yes")).clicked() {
          self.delete_hook_show = false;
          self.remove_hook_data();
          self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          modal.close();
          self
            .toast
            .success(t!("Localization files successfully deleted"));
        };
      });
    });
    modal.open();
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
    let _ = std::fs::write(self.bin.dir.join(PATH_FONT), &ORIGINAL_FONT);
    let _ = std::fs::remove_file(self.bin.lib_path.clone());
    let _ = std::fs::remove_dir_all(self.bin.dir.join("dfint-data"));
  }
}
