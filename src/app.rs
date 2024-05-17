use eframe::egui;
use std::path::PathBuf;

use crate::{
  constants::*,
  dict_metadata::DictMetadata,
  hook_metadata::HookMetadata,
  localization::{t, LOCALE},
  logic::Message,
  persistent::Store,
  thread_pool::ThreadPool,
  utils::*,
};

pub struct App {
  pub pool: ThreadPool<Message>,
  pub toast: egui_notify::Toasts,
  pub open_file_dialog: Option<egui_file::FileDialog>,
  pub opened_file: Option<PathBuf>,
  pub delete_old_data_show: bool,
  pub delete_hook_show: bool,
  pub on_start: bool,
  pub loading: u8,
  pub df_running: bool,
  pub selected_language: String,
  pub ui_locale: String,
  pub df_os: OS,
  pub df_dir: Option<PathBuf>,
  pub df_bin: Option<PathBuf>,
  pub df_checksum: u32,
  pub hook_checksum: u32,
  pub dict_checksum: u32,
  pub hook_metadata: HookMetadata,
  pub dict_metadata: DictMetadata,
}

impl Default for App {
  fn default() -> Self {
    let (df_bin, selected_language, hook_metadata, dict_metadata) = match Store::load() {
      Ok(store) => (
        Some(PathBuf::from(store.df_bin)),
        store.selected_language,
        HookMetadata::from_store(store.hook_manifest, store.vec_hook_manifests),
        DictMetadata::from_store(store.dict_manifest, store.vec_dict_manifests),
      ),
      Err(_) => (
        scan_df(),
        String::from("None"),
        HookMetadata::new(),
        DictMetadata::new(),
      ),
    };

    Self {
      pool: ThreadPool::new(),
      toast: egui_notify::Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
      open_file_dialog: None,
      opened_file: None,
      delete_old_data_show: false,
      delete_hook_show: false,
      on_start: true,
      loading: 0,
      df_running: is_df_running(),
      selected_language,
      ui_locale: LOCALE.read().current_locale(),
      df_os: df_os_by_bin(&df_bin),
      df_dir: df_dir_by_bin(&df_bin),
      df_bin,
      df_checksum: 0,
      hook_checksum: 0,
      dict_checksum: 0,
      hook_metadata,
      dict_metadata,
    }
  }
}

impl eframe::App for App {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // guards
    if self.df_running {
      self.guard(
        ctx,
        "df_is_running",
        &t!("Dwarf Fortress is running. Close it before using the installer."),
      );
      return;
    }
    // handle incoming messages from thread pool
    self.update_state();
    // on first update (on startup)
    self.on_start(ctx);
    // if file dialog opened
    self.opened_file_dialog(ctx);
    // if delete old data dialog opened
    if self.delete_old_data_show {
      self.delete_old_hook_dialog(ctx)
    }
    // if delete hook dialog opened
    if self.delete_hook_show {
      self.delete_hook_dialog(ctx)
    }

    // close event
    ctx.input(|i| {
      if i.viewport().close_requested() {
        self.on_close();
      }
    });

    // UI block
    // status bar
    egui::TopBottomPanel::bottom("status")
      .min_height(25.)
      .show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
          ui.add(
            egui::Image::new(GITHUB_ICON.to_owned())
              .max_height(15.)
              .max_width(15.),
          );
          ui.hyperlink_to(t!("Report bug"), URL_BUGS);
          ui.add(
            egui::Image::new(TRANSIFEX_ICON.to_owned())
              .max_height(15.)
              .max_width(15.),
          );
          ui.hyperlink_to(t!("Help with translation"), URL_TRANSIFEX);
          ui.label(format!("v{VERSION}"));
          ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_source("locale")
              .selected_text(&self.ui_locale)
              .width(50.)
              .show_ui(ui, |ui| {
                let mut lock = LOCALE.write();
                for item in lock.locales() {
                  if ui
                    .selectable_value(&mut self.ui_locale, item.clone(), item.clone())
                    .clicked()
                  {
                    lock.set(&item)
                  }
                }
              });
          });
        });
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

      ui.horizontal(|ui| {
        ui.heading(t!("Hook"));
        // cheksums without lozalization files
        if self.hook_checksum != 4282505490 || self.dict_checksum != 1591420153 {
          ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            let button = ui
              .add_sized([20., 20.], egui::Button::new("🗑"))
              .on_hover_text(t!("Delete localization files"));
            if button.clicked() {
              self.delete_hook_show = true
            }
          });
        }
      });
      ui.separator();

      egui::Grid::new("hook grid")
        .num_columns(4)
        .min_col_width(150.)
        .spacing([5., 5.])
        .striped(true)
        .show(ui, |ui| {
          ui.label(t!("Version"));
          ui.label(self.hook_checksum.to_string());

          if self.hook_metadata.manifest.checksum == 0 {
            ui.label("?");
          } else {
            ui.label(self.hook_metadata.manifest.checksum.to_string());
          }

          ui.label(
            match (
              self.hook_metadata.manifest.df == self.df_checksum,
              self.hook_metadata.manifest.checksum == self.hook_checksum,
              self.hook_metadata.manifest.checksum == 0,
              self.hook_metadata.vec_manifests.len() == 0,
            ) {
              (_, _, true, true) => format!("✖ {}", t!("hook data was not loaded")),
              (false, _, _, _) => format!("✖ {}", t!("this DF version is not supported")),
              (true, true, _, _) => format!("✅ {}", t!("up-to-date")),
              (true, false, _, _) => format!("⚠ {}", t!("update available")),
            },
          );
          ui.end_row();
        });
      ui.add_space(20.);

      ui.heading(t!("Dictionary"));
      ui.separator();

      egui::Grid::new("dictionary grid")
        .num_columns(4)
        .min_col_width(150.)
        .spacing([5., 5.])
        .striped(true)
        .show(ui, |ui| {
          egui::ComboBox::from_id_source("languages")
            .selected_text(&self.selected_language)
            .width(140.)
            .show_ui(ui, |ui| {
              for item in self.dict_metadata.vec_manifests.clone().iter() {
                if ui
                  .selectable_value(
                    &mut self.selected_language,
                    item.language.clone(),
                    item.language.clone(),
                  )
                  .clicked()
                {
                  if self.selected_language != "None" {
                    self
                      .dict_metadata
                      .pick_language(self.selected_language.clone())
                  }
                };
              }
            });
          ui.label(self.dict_checksum.to_string());
          if self.dict_metadata.manifest.checksum == 0 {
            ui.label("?");
          } else {
            ui.label(self.dict_metadata.manifest.checksum.to_string());
          }
          ui.label(
            match (
              self.dict_metadata.manifest.checksum == self.dict_checksum,
              self.selected_language == "None",
            ) {
              (true, false) => format!("✅ {}", t!("up-to-date")),
              (false, false) => format!("⚠ {}", t!("update available")),
              (_, true) => format!("⚠ {}", t!("choose language")),
            },
          );
          ui.end_row();
        });
      ui.add_space(20.);

      if (self.hook_metadata.manifest.df == self.df_checksum
        && self.hook_metadata.manifest.checksum != self.hook_checksum)
        || (self.dict_metadata.manifest.checksum != self.dict_checksum && self.selected_language != "None")
      {
        ui.style_mut().text_styles.insert(
          egui::TextStyle::Button,
          egui::FontId::new(20., eframe::epaint::FontFamily::Proportional),
        );
        ui.vertical_centered(|ui| {
          if self.loading > 0 {
            ui.add(egui::Spinner::new().size(40.));
          } else {
            let button = ui.add_sized([130., 40.], egui::Button::new(t!("Update")));
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
