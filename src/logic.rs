use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;

use crate::{
  app::App,
  constants::*,
  localization::{t, LOCALE},
  state::{read, write, STATE},
  utils::*,
};

macro_rules! spawn {
  ($l:expr) => {
    std::thread::spawn(move || {
      $l;
    });
  };
}
macro_rules! error {
  ($l:expr) => {
    write!(notify, (Notification::Error, $l))
  };
}
macro_rules! success {
  ($l:expr) => {
    write!(notify, (Notification::Success, $l))
  };
}

impl App {
  pub fn file_dialog(&self, df_dir: Option<PathBuf>) -> Option<egui_file::FileDialog> {
    let mut dialog = egui_file::FileDialog::open_file(self.opened_file.clone())
      .filter(Box::new(|path| is_df_bin(path)))
      .resizable(false)
      .show_rename(false)
      .show_new_folder(false)
      .title(&t!("Open Dwarf Fortress executable"))
      .default_size([720., 381.]);
    dialog.set_path(df_dir.unwrap_or(std::env::current_dir().unwrap()));
    dialog.open();
    Some(dialog)
  }

  pub fn opened_file_dialog(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    if let Some(dialog) = &mut self.open_file_dialog {
      if dialog.state() == egui_file::State::Closed && self.df_os == OS::None {
        frame.close();
      }
      if dialog.show(ctx).selected() {
        if let Some(file) = dialog.path() {
          self.df_bin = Some(file.to_path_buf());
          self.df_os = df_os_by_bin(&self.df_bin);
          self.df_dir = Some(file.parent().unwrap().to_path_buf());
          self.df_checksum = df_checksum(&self.df_bin, self.df_os).unwrap_or(0);
          self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
          self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
          // self.dfhack_installed = is_dfhack_installed(&self.df_dir);
          let manifests = read!(vec_hook_manifests).clone();
          if let Some(manifest) = get_manifest_by_df(self.df_checksum, manifests) {
            write!(hook_manifest, manifest);
          }
          self.delete_old_data_check();
        }
      }
    }
  }

  pub fn on_start(&mut self, ctx: &egui::Context) {
    if self.on_start {
      self.on_start = false;

      let df_checksum = df_checksum(&self.df_bin, self.df_os).unwrap_or(0);
      self.df_checksum = df_checksum;
      self.hook_checksum = self.local_hook_checksum().unwrap_or(0);

      spawn!({
        match fetch_manifest::<HookManifest>(URL_HOOK_MANIFEST) {
          Ok(manifests) => {
            write!(vec_hook_manifests, manifests.clone());
            if let Some(manifest) = get_manifest_by_df(df_checksum, manifests) {
              write!(hook_manifest, manifest);
            } else {
              if df_checksum != 0 {
                error!(t!("This DF version is not supported"));
              }
            }
          }
          Err(_) => {
            error!(t!("Unable to fetch hook metadata..."));
          }
        }
      });

      let selected_language = self.selected_language.clone();
      self.dict_checksum = self.local_dict_checksum().unwrap_or(0);

      spawn!({
        match fetch_manifest::<DictManifest>(URL_DICT_MANIFEST) {
          Ok(manifests) => {
            write!(vec_dict_manifests, manifests.clone());
            if let Some(manifest) = get_manifest_by_language(selected_language, manifests) {
              write!(dict_manifest, manifest);
            }
          }
          Err(_) => {
            error!(t!("Unable to fetch dictionary metadata..."));
          }
        }
      });

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

  pub fn notify(&mut self) {
    let (level, message) = read!(notify).clone();
    if level != Notification::None {
      match level {
        Notification::Error => {
          self.toast.error(message);
        }
        Notification::Warning => {
          self.toast.warning(message);
        }
        Notification::Info => {
          self.toast.info(message);
        }
        Notification::Success => {
          self.toast.success(message);
        }
        Notification::None => (),
      }
      write!(notify, (Notification::None, "".into()));
    }
  }

  pub fn recalculate_checksum(&mut self) {
    let hc = read!(recalculate_hook_checksum);
    if hc {
      write!(recalculate_hook_checksum, false);
      self.hook_checksum = self.local_hook_checksum().unwrap_or(0);
    }
    let dc = read!(recalculate_dict_checksum);
    if dc {
      write!(recalculate_dict_checksum, false);
      self.dict_checksum = self.local_dict_checksum().unwrap_or(0);
    }
  }

  pub fn guard(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, name: &str, text: &str) {
    egui::CentralPanel::default().show(ctx, |_ui| {
      let modal = egui_modal::Modal::new(ctx, name);
      modal.show(|ui| {
        modal.title(ui, t!("Warning"));
        modal.frame(ui, |ui| {
          modal.body_and_icon(ui, t!(text), egui_modal::Icon::Info);
        });
        modal.buttons(ui, |ui| {
          if modal.caution_button(ui, "Ok").clicked() {
            frame.close();
          };
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
          self.toast.success(t!("Localization files successfully deleted"));
        };
      });
    });
    modal.open();
  }

  pub fn update_data(&mut self) {
    let _ = self.create_dir_if_not_exist();

    let hook_manifest = read!(hook_manifest).clone();
    let df_dir = self.df_dir.clone().unwrap();
    let lib = self.get_lib_path("dfhooks_dfint").unwrap();
    let dfhooks = self.get_lib_path("dfhooks").unwrap();
    if hook_manifest.df == self.df_checksum && hook_manifest.checksum != self.hook_checksum {
      let loading = read!(loading);
      write!(loading, loading + 1);
      spawn!({
        let r1 = download_to_file(&hook_manifest.lib, &lib);
        let r2 = download_to_file(&hook_manifest.config, &df_dir.join(PATH_CONFIG));
        let r3 = download_to_file(&hook_manifest.offsets, &df_dir.join(PATH_OFFSETS));
        let r4 = download_to_file(&hook_manifest.dfhooks, &dfhooks);
        let loading = read!(loading);
        if r1.is_ok() && r2.is_ok() && r3.is_ok() && r4.is_ok() {
          write!(recalculate_hook_checksum, true);
          success!(t!("Hook updated"));
        } else {
          error!(t!("Unable to update hook"));
        }
        write!(loading, loading - 1);
      });
    }

    let dict_manifest = read!(dict_manifest).clone();
    let df_dir = self.df_dir.clone().unwrap();
    if dict_manifest.checksum != self.dict_checksum && self.selected_language != "None" {
      let loading = read!(loading);
      write!(loading, loading + 1);
      spawn!({
        let r1 = download_to_file(&dict_manifest.csv, &df_dir.join(PATH_DICT));
        let r2 = download_to_file(&dict_manifest.font, &df_dir.join(PATH_FONT));
        let r3 = download_to_file(&dict_manifest.encoding, &df_dir.join(PATH_ENCODING));
        let loading = read!(loading);
        if r1.is_ok() && r2.is_ok() && r3.is_ok() {
          write!(recalculate_dict_checksum, true);
          success!(t!("Dictionary updated"));
        } else {
          error!(t!("Unable to update dictionary"));
        }
        write!(loading, loading - 1);
      });
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
