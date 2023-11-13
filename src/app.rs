use eframe::egui;
use std::path::PathBuf;

use crate::{
  constants::*,
  localization::{t, LOCALE},
  persistent,
  state::{read, write, STATE},
  utils::*,
};

pub struct App {
  pub toast: egui_notify::Toasts,
  pub open_file_dialog: Option<egui_file::FileDialog>,
  pub opened_file: Option<PathBuf>,
  pub delete_old_data_show: bool,
  pub on_start: bool,
  pub df_running: bool,
  pub selected_language: String,
  pub df_os: OS,
  pub df_dir: Option<PathBuf>,
  pub df_bin: Option<PathBuf>,
  pub df_checksum: u32,
  pub hook_checksum: u32,
  pub dict_checksum: u32,
}

impl Default for App {
  fn default() -> Self {
    let (df_bin, selected_language) = match persistent::load() {
      Ok(store) => {
        write!(hook_manifest, store.hook_manifest);
        write!(dict_manifest, store.dict_manifest);
        (Some(PathBuf::from(store.df_bin)), store.selected_language)
      }
      Err(_) => (scan_df(), String::from("None")),
    };

    Self {
      toast: egui_notify::Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
      open_file_dialog: None,
      opened_file: None,
      delete_old_data_show: false,
      on_start: true,
      df_running: is_df_running(),
      selected_language,
      df_os: df_os_by_bin(&df_bin),
      df_dir: df_dir_by_bin(&df_bin),
      df_bin,
      df_checksum: 0,
      hook_checksum: 0,
      dict_checksum: 0,
    }
  }
}

impl eframe::App for App {
  fn on_close_event(&mut self) -> bool {
    if self.df_bin.is_some() {
      let _ = persistent::save(persistent::Store {
        df_bin: self.df_bin.clone().unwrap().as_path().display().to_string(),
        hook_manifest: read!(hook_manifest).clone(),
        vec_hook_manifests: read!(vec_hook_manifests).clone(),
        dict_manifest: read!(dict_manifest).clone(),
        vec_dict_manifests: read!(vec_dict_manifests).clone(),
        selected_language: self.selected_language.clone(),
      });
    }
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
    self.opened_file_dialog(ctx, frame);
    // if delete old data dialog opened
    if self.delete_old_data_show {
      self.delete_old_hook_dialog(ctx)
    }

    // UI block
    // status bar
    egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
      ui.horizontal_centered(|ui| {
        ui.add(egui::Image::new(BOOSTY_ICON.to_owned()).max_height(15.).max_width(15.));
        ui.hyperlink_to(t!("support"), URL_BOOSTY);
        ui.add(egui::Image::new(GITHUB_ICON.to_owned()).max_height(15.).max_width(15.));
        ui.hyperlink_to(t!("report bug"), URL_BUGS);
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
          ui.label(t!("Path"));
          ui.label(match &self.df_bin {
            Some(pathbuf) => pathbuf.as_path().display().to_string(),
            None => "None".to_owned(),
          });
          if ui.small_button("🔍").clicked() {
            let dir = self.df_dir.clone();
            self.open_file_dialog = self.file_dialog(dir);
          };
          ui.end_row();
          ui.label(t!("OS"));
          ui.label(self.df_os.to_string());
          ui.end_row();
          ui.label(t!("Checksum"));
          ui.label(format!("{:x}", self.df_checksum));
          ui.end_row();
        });
      ui.add_space(20.);

      // if binary not valid, do not render main section
      if self.df_os == OS::None {
        return;
      }

      ui.heading(t!("Hook"));
      ui.separator();

      egui::Grid::new("hook grid").num_columns(4).min_col_width(150.).spacing([5., 5.]).striped(true).show(ui, |ui| {
        let hook_manifest = read!(hook_manifest).clone();
        ui.label(t!("Version"));
        ui.label(self.hook_checksum.to_string());
        ui.label(hook_manifest.version.to_string());
        ui.label(
          match (
            hook_manifest.df == self.df_checksum,
            hook_manifest.version == self.hook_checksum,
          ) {
            (true, true) => format!("✅ {}", t!("up-to-date")),
            (true, false) => format!("⚠ {}", t!("update available")),
            (false, _) => format!("✖ {}", t!("this DF version is not supported")),
          },
        );
        ui.end_row();
      });
      ui.add_space(20.);

      ui.heading(t!("Dictionary"));
      ui.separator();

      egui::Grid::new("dictionary grid").num_columns(4).min_col_width(150.).spacing([5., 5.]).striped(true).show(
        ui,
        |ui| {
          egui::ComboBox::from_id_source("languages").selected_text(&self.selected_language).width(140.).show_ui(
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
                    let manifest = get_manifest_by_language(self.selected_language.clone(), manifests.clone());
                    write!(dict_manifest, manifest.unwrap());
                  }
                };
              }
            },
          );
          let dict_manifest = &read!(dict_manifest);
          ui.label(self.dict_checksum.to_string());
          ui.label(dict_manifest.version.to_string());
          ui.label(
            match (
              dict_manifest.version == self.dict_checksum,
              self.selected_language == "None",
            ) {
              (true, false) => format!("✅ {}", t!("up-to-date")),
              (false, false) => format!("⚠ {}", t!("update available")),
              (_, true) => format!("⚠ {}", t!("choose language")),
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
            let button = ui.add_sized([120., 40.], egui::Button::new(t!("Update")));
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
