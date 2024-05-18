use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;

use crate::{
  app::App,
  constants::*,
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
}

impl App {
  pub fn file_dialog(&self, df_dir: Option<PathBuf>) -> Option<egui_file::FileDialog> {
    let mut dialog = egui_file::FileDialog::open_file(self.opened_file.clone())
      .show_files_filter(Box::new(|path| is_df_bin(path)))
      .resizable(false)
      .show_rename(false)
      .show_new_folder(false)
      .title(&t!("Open Dwarf Fortress executable"))
      .default_size([720., 381.]);
    dialog.set_path(df_dir.unwrap_or(std::env::current_dir().unwrap()));
    dialog.open();
    Some(dialog)
  }

  pub fn opened_file_dialog(&mut self, ctx: &egui::Context) {
    if let Some(dialog) = &mut self.open_file_dialog {
      if dialog.state() == egui_file::State::Closed && self.df_os == OS::None {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
      }
      if dialog.show(ctx).selected() {
        if let Some(file) = dialog.path() {
          self.df_bin = Some(file.to_path_buf());
          self.df_os = df_os_by_bin(&self.df_bin);
          self.df_dir = Some(file.parent().unwrap().to_path_buf());
          self.df_checksum = df_checksum(&self.df_bin, self.df_os).unwrap_or(0);
          self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          self.hook_metadata.pick_df_checksum(self.df_checksum);
          self.delete_old_data_check();
        }
      }
    }
  }

  pub fn on_close(&mut self) {
    if self.df_bin.is_some() {
      let _ = Store {
        df_bin: self.df_bin.clone().unwrap().as_path().display().to_string(),
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
            if self.hook_metadata.manifest.checksum == 0 && self.df_bin.is_some() {
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
      }
    }
  }

  pub fn on_start(&mut self, ctx: &egui::Context) {
    if self.on_start {
      self.on_start = false;

      self.df_checksum = df_checksum(&self.df_bin, self.df_os).unwrap_or(0);
      self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
      self.pool.execute(
        HookMetadata::from_url(URL_HOOK_MANIFEST, Some(self.df_checksum)),
        Message::HookMetadataLoaded,
      );

      self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
      self.pool.execute(
        DictMetadata::from_url(URL_DICT_MANIFEST, Some(self.selected_language.clone())),
        Message::DictMetadataLoaded,
      );

      if self.df_os == OS::None {
        egui::CentralPanel::default().show(ctx, |_ui| {
          let dir = self.df_dir.clone();
          self.open_file_dialog = self.file_dialog(dir);
        });
        return;
      }

      self.delete_old_data_check();
    }
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

  pub fn delete_old_data_check(&mut self) {
    if self.df_dir.is_none() {
      return;
    }
    let launcher = self.df_dir.clone().unwrap().join("dfint_launcher.exe");
    let old_data = self.df_dir.clone().unwrap().join("dfint_data");
    if launcher.exists() || old_data.exists() {
      self.delete_old_data_show = true;
    }
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
    let _ = self.create_dir_if_not_exist();

    let hook_manifest = self.hook_metadata.manifest.clone();
    let df_dir = self.df_dir.clone().unwrap();
    if hook_manifest.df == self.df_checksum && hook_manifest.checksum != self.hook_checksum {
      self.loading += 1;
      self.pool.execute(
        batch_download_to_file(vec![
          (hook_manifest.lib, self.get_lib_path("dfhooks_dfint").unwrap()),
          (hook_manifest.config, df_dir.join(PATH_CONFIG)),
          (hook_manifest.offsets, df_dir.join(PATH_OFFSETS)),
          (hook_manifest.dfhooks, self.get_lib_path("dfhooks").unwrap()),
        ]),
        Message::HookUpdated,
      );
    }

    let dict_manifest = self.dict_metadata.manifest.clone();
    let df_dir = self.df_dir.clone().unwrap();
    if dict_manifest.checksum != self.dict_checksum && self.selected_language != "None" {
      self.loading += 1;
      self.pool.execute(
        batch_download_to_file(vec![
          (dict_manifest.csv, df_dir.join(PATH_DICT)),
          (dict_manifest.font, df_dir.join(PATH_FONT)),
          (dict_manifest.encoding, df_dir.join(PATH_ENCODING)),
        ]),
        Message::DictUpdated,
      );
    }
  }

  pub fn get_lib_path(&self, name: &str) -> Option<PathBuf> {
    match (&self.df_dir, self.df_os) {
      (Some(pathbuf), OS::Windows) => Some(pathbuf.join(format!("{name}.dll"))),
      (Some(pathbuf), OS::Linux) => Some(pathbuf.join(format!("lib{name}.so"))),
      (_, _) => None,
    }
  }

  pub fn local_hook_checksum(&self) -> Result<u32> {
    match &self.df_dir {
      Some(pathbuf) => checksum_for_files(vec![
        self.get_lib_path("dfhooks_dfint"),
        Some(pathbuf.join(PATH_CONFIG)),
        Some(pathbuf.join(PATH_OFFSETS)),
        self.get_lib_path("dfhooks"),
      ]),
      None => Ok(0),
    }
  }

  pub fn local_dict_checksum(&self) -> Result<u32> {
    match &self.df_dir {
      Some(pathbuf) => checksum_for_files(vec![
        Some(pathbuf.join(PATH_DICT)),
        Some(pathbuf.join(PATH_FONT)),
        Some(pathbuf.join(PATH_ENCODING)),
      ]),
      None => Ok(0),
    }
  }

  pub fn create_dir_if_not_exist(&self) -> Result<()> {
    if let Some(pathbuf) = &self.df_dir {
      std::fs::create_dir_all(pathbuf.join(PATH_DATA))?;
    }
    Ok(())
  }

  pub fn remove_old_data(&self) {
    if let Some(pathbuf) = &self.df_dir {
      let _ = std::fs::remove_file(pathbuf.join("dfint_launcher.exe"));
      let _ = std::fs::remove_dir_all(pathbuf.join("dfint_data"));
    }
  }

  pub fn remove_hook_data(&self) {
    if let Some(pathbuf) = &self.df_dir {
      let _ = std::fs::write(pathbuf.join(PATH_FONT), &ORIGINAL_FONT);
      let _ = std::fs::remove_file(self.get_lib_path("dfhooks_dfint").unwrap());
      let _ = std::fs::remove_dir_all(pathbuf.join("dfint-data"));
    }
  }
}
