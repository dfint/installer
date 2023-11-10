use eframe::egui;
use std::path::PathBuf;

use crate::constants::*;
// use crate::localization::{t, LOCALE};
use crate::logic::*;
use crate::persistent;
use crate::state::{read, write, STATE};

macro_rules! spawn {
  ($l:expr) => {
    std::thread::spawn(move || {
      $l;
    });
  };
}
pub(crate) use spawn;

macro_rules! error {
  ($l:expr) => {
    write!(notify, (Notification::Error, $l.into()))
  };
}
macro_rules! _info {
  ($l:expr) => {
    write!(notify, (Notification::Info, $l.into()))
  };
}
macro_rules! _warning {
  ($l:expr) => {
    write!(notify, (Notification::warning, $l.into()))
  };
}
macro_rules! success {
  ($l:expr) => {
    write!(notify, (Notification::Success, $l.into()))
  };
}

pub struct App {
  // internal
  toast: egui_notify::Toasts,
  open_file_dialog: Option<egui_file::FileDialog>,
  opened_file: Option<PathBuf>,
  delete_old_data_show: bool,
  on_start: bool,
  df_running: bool,
  selected_language: String,
  // data
  df_os: OS,
  df_bin: Option<PathBuf>,
  df_dir: Option<PathBuf>,
  df_checksum: u32,
  hook_checksum: u32,
  dict_checksum: u32,
}

impl Default for App {
  fn default() -> Self {
    let df_bin: Option<PathBuf>;
    let selected_language: String;

    match persistent::load() {
      Ok(store) => {
        df_bin = Some(PathBuf::from(store.df_bin));
        selected_language = store.selected_language;
        write!(hook_manifest, store.hook_manifest);
        write!(dict_manifest, store.dict_manifest);
      }
      Err(_) => {
        df_bin = scan_df();
        selected_language = String::from("None");
      }
    };

    let df_dir = df_dir_by_bin(&df_bin);
    let df_os = df_os_by_bin(&df_bin);

    Self {
      toast: egui_notify::Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
      open_file_dialog: None,
      opened_file: None,
      delete_old_data_show: false,
      on_start: true,
      df_running: is_df_running(),
      selected_language,

      df_os,
      df_bin,
      df_dir,
      df_checksum: 0,
      hook_checksum: 0,
      dict_checksum: 0,
    }
  }
}

impl eframe::App for App {
  fn on_close_event(&mut self) -> bool {
    let _ = persistent::save(persistent::Store {
      df_bin: self.df_bin.clone().unwrap().as_path().display().to_string(),
      hook_manifest: read!(hook_manifest).clone(),
      vec_hook_manifests: read!(vec_hook_manifests).clone(),
      dict_manifest: read!(dict_manifest).clone(),
      vec_dict_manifests: read!(vec_dict_manifests).clone(),
      selected_language: self.selected_language.clone(),
    });
    true
  }

  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    // Logic block

    // df running guard
    if self.df_running {
      self.df_running_guard(ctx, frame);
      return;
    }
    // on first update (on startup)
    self.on_start(ctx);
    // trigger pending notification
    self.notify();
    // pending checksums update
    self.recalculate_checksum();
    // if file dialog opened
    self.opened_file_dialog(ctx);
    // if delete old data dialog opened
    if self.delete_old_data_show {
      self.delete_old_hook_dialog(ctx)
    }

    // UI block

    // status bar
    egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
      ui.horizontal_centered(|ui| {
        ui.add(egui::Image::new(BOOSTY_ICON.to_owned()).max_height(15.).max_width(15.));
        ui.hyperlink_to("support", "https://boosty.to/dfrus");
        ui.add(egui::Image::new(GITHUB_ICON.to_owned()).max_height(15.).max_width(15.));
        ui.hyperlink_to("report bug", "https://github.com/dfint");
      })
    });

    egui::CentralPanel::default().show(ctx, |ui| {
      ui.add_space(5.);
      ui.heading("Dwarf Fortress");
      ui.separator();
      egui::Grid::new("executable grid")
        .num_columns(2)
        .min_col_width(150.)
        .max_col_width(450.)
        .spacing([5., 5.])
        .striped(true)
        .show(ui, |ui| {
          ui.label("Path");
          ui.label(match self.df_bin.clone() {
            Some(pathbuf) => pathbuf.as_path().display().to_string(),
            None => "None".to_owned(),
          });
          if ui.small_button("🔍").clicked() {
            let dir = self.df_dir.clone();
            self.open_file_dialog = self.file_dialog(dir);
          };
          ui.end_row();
          ui.label("OS");
          ui.label(format!("{}", self.df_os));
          ui.end_row();
          ui.label("Checksum");
          ui.label(format!("{:x}", self.df_checksum));
          ui.end_row();
        });
      ui.add_space(20.);

      // if binary not valid, do not render main section
      if self.df_os == OS::None {
        return;
      }

      ui.heading("Hook");
      ui.separator();

      egui::Grid::new("hook grid").num_columns(4).min_col_width(150.).spacing([5., 5.]).striped(true).show(ui, |ui| {
        let hook_manifest = read!(hook_manifest).clone();
        ui.label("Version");
        ui.label(self.hook_checksum.to_string());
        ui.label(hook_manifest.version.to_string());
        ui.label(
          match (
            hook_manifest.df == self.df_checksum,
            hook_manifest.version == self.hook_checksum,
          ) {
            (true, true) => "✅ up to date",
            (true, false) => "⚠ update available",
            (false, _) => "✖ DF version not supported",
          },
        );
        ui.end_row();
      });
      ui.add_space(20.);

      ui.heading("Dictionary");
      ui.separator();

      egui::Grid::new("dictionary grid").num_columns(4).min_col_width(150.).spacing([5., 5.]).striped(true).show(
        ui,
        |ui| {
          egui::ComboBox::from_id_source("langs").selected_text(self.selected_language.clone()).width(140.).show_ui(
            ui,
            |ui| {
              let manifests = read!(vec_dict_manifests).clone();
              for item in manifests.iter() {
                if ui
                  .selectable_value(
                    &mut self.selected_language,
                    item.language.clone(),
                    item.language.clone(),
                  )
                  .clicked()
                {
                  if self.selected_language != "None" {
                    let lang = self.selected_language.clone();
                    let manifest = get_manifest_by_language(lang.clone(), manifests.clone());
                    write!(dict_manifest, manifest.unwrap());
                  }
                };
              }
            },
          );
          let dict_manifest = read!(dict_manifest).clone();
          ui.label(self.dict_checksum.to_string());
          ui.label(dict_manifest.version.to_string());
          ui.label(
            match (
              dict_manifest.version == self.dict_checksum,
              self.selected_language == "None",
            ) {
              (true, false) => "✅ up to date",
              (false, false) => "⚠ update available",
              (_, true) => "⚠ choose language",
            },
          );
          ui.end_row();
        },
      );
      ui.add_space(20.);

      let hook_manifest = read!(hook_manifest).clone();
      let dict_manifest = read!(dict_manifest).clone();

      if (hook_manifest.df == self.df_checksum && hook_manifest.version != self.hook_checksum)
        || (dict_manifest.version != self.dict_checksum && self.selected_language != "None")
      {
        ui.style_mut().text_styles.insert(
          egui::TextStyle::Button,
          egui::FontId::new(20., eframe::epaint::FontFamily::Proportional),
        );
        ui.vertical_centered(|ui| {
          let loading = read!(loading);
          if loading > 0 {
            ui.add(egui::Spinner::new().size(40.));
          } else {
            let button = ui.add_sized([120., 40.], egui::Button::new("Update"));
            if button.clicked() {
              self.update_data();
            }
          }
        });
      }
    });

    self.toast.show(ctx)
  }
}

trait Logic {
  fn file_dialog(&self, df_dir: Option<PathBuf>) -> Option<egui_file::FileDialog>;
  fn opened_file_dialog(&mut self, ctx: &egui::Context);
  fn on_start(&mut self, ctx: &egui::Context);
  fn notify(&mut self);
  fn recalculate_checksum(&mut self);
  fn df_running_guard(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame);
  fn delete_old_hook_dialog(&mut self, ctx: &egui::Context);
  fn delete_old_data_check(&mut self);
  fn update_data(&mut self);
}

impl Logic for App {
  fn file_dialog(&self, df_dir: Option<PathBuf>) -> Option<egui_file::FileDialog> {
    let mut dialog = egui_file::FileDialog::open_file(self.opened_file.clone())
      .filter(Box::new(|path| is_df_bin(path)))
      .resizable(false)
      .show_rename(false)
      .show_new_folder(false)
      .title("open Dwarf Fortress executable")
      .default_size([700., 381.]);
    dialog.set_path(df_dir.unwrap_or(std::env::current_dir().unwrap()));
    dialog.open();
    Some(dialog)
  }

  fn opened_file_dialog(&mut self, ctx: &egui::Context) {
    if let Some(dialog) = &mut self.open_file_dialog {
      if dialog.show(ctx).selected() {
        if let Some(file) = dialog.path() {
          self.df_bin = Some(file.to_path_buf());
          self.df_os = df_os_by_bin(&self.df_bin);
          self.df_dir = Some(file.parent().unwrap().to_path_buf());
          self.df_checksum = df_checksum(&self.df_bin, self.df_os).unwrap_or(0);
          self.hook_checksum = local_hook_checksum(&get_lib_path(&self.df_dir, self.df_os), &self.df_dir).unwrap_or(0);
          self.dict_checksum = local_dict_checksum(&self.df_dir).unwrap_or(0);
          let manifests = read!(vec_hook_manifests).clone();
          if let Some(manifest) = get_manifest_by_df(self.df_checksum, manifests) {
            write!(hook_manifest, manifest);
          }
          self.delete_old_data_check();
        }
      }
    }
  }

  fn on_start(&mut self, ctx: &egui::Context) {
    if self.on_start {
      self.on_start = false;

      let df_checksum = df_checksum(&self.df_bin, self.df_os).unwrap_or(0);
      self.df_checksum = df_checksum;
      self.hook_checksum = local_hook_checksum(&get_lib_path(&self.df_dir, self.df_os), &self.df_dir).unwrap_or(0);

      spawn!({
        match fetch_hook_manifest() {
          Ok(manifests) => {
            write!(vec_hook_manifests, manifests.clone());
            if let Some(manifest) = get_manifest_by_df(df_checksum, manifests) {
              write!(hook_manifest, manifest);
            } else {
              if df_checksum != 0 {
                error!("DF version not supported");
              }
            }
          }
          Err(_) => {
            error!("Unable to fetch hook metadata...");
          }
        }
      });

      let selected_language = self.selected_language.clone();
      self.dict_checksum = local_dict_checksum(&self.df_dir).unwrap_or(0);

      spawn!({
        match fetch_dict_manifest() {
          Ok(manifests) => {
            write!(vec_dict_manifests, manifests.clone());
            if let Some(manifest) = get_manifest_by_language(selected_language, manifests) {
              write!(dict_manifest, manifest);
            }
          }
          Err(_) => {
            error!("Unable to fetch dictionary metadata...");
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

  fn notify(&mut self) {
    let (level, message) = read!(notify);
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
      write!(notify, (Notification::None, ""));
    }
  }

  fn recalculate_checksum(&mut self) {
    let hc = read!(recalculate_hook_checksum);
    if hc {
      write!(recalculate_hook_checksum, false);
      self.hook_checksum = local_hook_checksum(&get_lib_path(&self.df_dir, self.df_os), &self.df_dir).unwrap_or(0);
    }
    let dc = read!(recalculate_dict_checksum);
    if dc {
      write!(recalculate_dict_checksum, false);
      self.dict_checksum = local_dict_checksum(&self.df_dir).unwrap_or(0);
    }
  }

  fn df_running_guard(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ctx, |_ui| {
      let modal = egui_modal::Modal::new(ctx, "df_is_running");
      modal.show(|ui| {
        modal.title(ui, "Warning");
        modal.frame(ui, |ui| {
          modal.body_and_icon(
            ui,
            "Dwarf Fortress is running, close it before using the installer.",
            egui_modal::Icon::Info,
          );
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

  fn delete_old_data_check(&mut self) {
    if self.df_dir.is_none() {
      return;
    }
    let launcher = self.df_dir.clone().unwrap().join("dfint_launcher.exe");
    let old_data = self.df_dir.clone().unwrap().join("dfint_data");
    if launcher.exists() || old_data.exists() {
      self.delete_old_data_show = true;
    }
  }

  fn delete_old_hook_dialog(&mut self, ctx: &egui::Context) {
    let modal = egui_modal::Modal::new(ctx, "delete_old_data");
    modal.show(|ui| {
      modal.title(ui, "Warning");
      modal.frame(ui, |ui| {
        modal.body_and_icon(
          ui,
          "Old version of translation stuff detected. It's better to delete it to avoid conflicts. Delete?",
          egui_modal::Icon::Info,
        );
      });
      modal.buttons(ui, |ui| {
        if modal.button(ui, "No").clicked() {
          self.delete_old_data_show = false;
          modal.close();
        };
        if modal.suggested_button(ui, "Yes").clicked() {
          self.delete_old_data_show = false;
          let _ = remove_old_data(&self.df_dir);
          modal.close();
          self.toast.success("Old data was deleted");
        };
      });
    });
    modal.open();
  }

  fn update_data(&mut self) {
    let _ = create_dir_if_not_exist(&self.df_dir);

    let hook_manifest = read!(hook_manifest).clone();
    let df_dir = self.df_dir.clone().unwrap();
    let df_os = self.df_os;
    if hook_manifest.df == self.df_checksum && hook_manifest.version != self.hook_checksum {
      let loading = read!(loading);
      write!(loading, loading + 1);
      spawn!({
        let r1 = download_to_file(&hook_manifest.lib, &get_lib_path(&Some(df_dir.clone()), df_os).unwrap());
        let r2 = download_to_file(&hook_manifest.config, &df_dir.join(PATH_CONFIG));
        let r3 = download_to_file(&hook_manifest.offsets, &df_dir.join(PATH_OFFSETS));
        let loading = read!(loading);
        if r1.is_ok() && r2.is_ok() && r3.is_ok() {
          write!(recalculate_hook_checksum, true);
          success!("Hook updated");
        } else {
          error!("Unable to update hook");
        }
        write!(loading, loading - 1);
      });
    }

    let dict_manifest = read!(dict_manifest).clone();
    let df_dir = self.df_dir.clone().unwrap();
    if dict_manifest.version != self.dict_checksum && self.selected_language != "None" {
      let loading = read!(loading);
      write!(loading, loading + 1);
      spawn!({
        let r1 = download_to_file(&dict_manifest.csv, &df_dir.join(PATH_DICT));
        let r2 = download_to_file(&dict_manifest.font, &df_dir.join(PATH_FONT));
        let r3 = download_to_file(&dict_manifest.encoding, &df_dir.join(PATH_ENCODING));
        let loading = read!(loading);
        if r1.is_ok() && r2.is_ok() && r3.is_ok() {
          write!(recalculate_dict_checksum, true);
          success!("Dictionary updated");
        } else {
          error!("Unable to update dictionary");
        }
        write!(loading, loading - 1);
      });
    }
  }
}
