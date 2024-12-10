use eframe::egui::{
  Align, Button, CentralPanel, ComboBox, Context, FontId, Grid, Image, Layout, Rect, Spinner, TextStyle,
  TopBottomPanel,
};
use std::path::PathBuf;

use crate::{
  constants::*,
  df_binary::DfBinary,
  dict_metadata::DictMetadata,
  hook_metadata::HookMetadata,
  localization::{t, LOCALE},
  logic::Message,
  thread_pool::ThreadPool,
};

#[derive(PartialEq)]
pub enum State {
  Startup,
  Loading,
  Idle,
}

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
  pub hook_checksum: u32,
  pub dict_checksum: u32,
  pub hook_metadata: HookMetadata,
  pub dict_metadata: DictMetadata,
  pub bin: DfBinary,
  pub state: State,
}

impl Default for App {
  fn default() -> Self {
    Self {
      pool: ThreadPool::new(),
      toast: egui_notify::Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
      open_file_dialog: None,
      opened_file: None,
      delete_old_data_show: false,
      delete_hook_show: false,
      on_start: true,
      loading: 0,
      df_running: false,
      selected_language: "None".to_string(),
      ui_locale: LOCALE.read().current_locale(),
      hook_checksum: 0,
      dict_checksum: 0,
      hook_metadata: HookMetadata::default(),
      dict_metadata: DictMetadata::default(),
      bin: DfBinary::default(),
      state: State::Startup,
    }
  }
}

impl eframe::App for App {
  fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
    // handle incoming messages from thread pool
    self.update_state();
    // close event
    ctx.input(|i| {
      if i.viewport().close_requested() {
        self.on_close();
      }
    });
    // guards
    if self.df_running {
      self.guard(
        ctx,
        "df_is_running",
        &t!("Dwarf Fortress is running. Close it before using the installer."),
      );
      return;
    }
    // on first update (on startup)
    if self.state == State::Startup {
      self.on_start();
    }
    // if binary not picked, force to do it
    if !self.bin.valid && self.open_file_dialog.is_none() && self.state == State::Idle {
      self.open_file_dialog = self.file_dialog(None);
    }
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
    // show loading on startup
    if self.state != State::Idle {
      CentralPanel::default().show(ctx, |ui| {
        ui.put(
          Rect::from_min_max([0., 0.].into(), [720., 450.].into()),
          Spinner::new().size(40.),
        );
      });
      return;
    }

    // UI block
    // status bar
    TopBottomPanel::bottom("status")
      .min_height(25.)
      .show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
          ui.add(
            Image::new(GITHUB_ICON.to_owned())
              .max_height(15.)
              .max_width(15.),
          );
          ui.hyperlink_to(t!("Report bug"), URL_BUGS);
          ui.add(
            Image::new(TRANSIFEX_ICON.to_owned())
              .max_height(15.)
              .max_width(15.),
          );
          ui.hyperlink_to(t!("Help with translation"), URL_TRANSIFEX);
          ui.label(format!("v{VERSION}"));
          ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ComboBox::from_id_source("locale")
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

    CentralPanel::default().show(ctx, |ui| {
      ui.add_space(5.);
      ui.heading("Dwarf Fortress");
      ui.separator();
      Grid::new("executable grid")
        .num_columns(2)
        .min_col_width(150.)
        .max_col_width(450.)
        .spacing([5., 5.])
        .striped(true)
        .show(ui, |ui| {
          ui.label(t!("Path"));
          ui.label(self.bin.to_string());
          if ui.small_button("🔍").clicked() {
            self.open_file_dialog = self.file_dialog(Some(self.bin.dir.clone()));
          };
          ui.end_row();
          ui.label(t!("OS"));
          ui.label(self.bin.os.to_string());
          ui.end_row();
          ui.label(t!("Checksum"));
          ui.label(format!("{:x}", self.bin.checksum));
          ui.end_row();
        });
      ui.add_space(20.);

      ui.horizontal(|ui| {
        ui.heading(t!("Hook"));
        // cheksums without lozalization files
        if self.hook_checksum != 4282505490 || self.dict_checksum != 1591420153 {
          ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            let button = ui
              .add_sized([20., 20.], Button::new("🗑"))
              .on_hover_text(t!("Delete localization files"));
            if button.clicked() {
              self.delete_hook_show = true
            }
          });
        }
      });
      ui.separator();

      Grid::new("hook grid")
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

          let (text, color) = match (
            self.hook_metadata.manifest.df == self.bin.checksum,
            self.hook_metadata.manifest.checksum == self.hook_checksum,
            self.hook_metadata.manifest.checksum == 0,
            self.hook_metadata.vec_manifests.len() == 0,
          ) {
            (_, _, true, true) => (format!("✖ {}", t!("hook data was not loaded")), COLOR_ERROR),
            (false, _, _, _) => (
              format!("✖ {}", t!("this DF version is not supported")),
              COLOR_ERROR,
            ),
            (true, true, _, _) => (format!("✅ {}", t!("up-to-date")), COLOR_UP_TO_DATE),
            (true, false, _, _) => (format!("⚠ {}", t!("update available")), COLOR_UPDATE_AVAILABLE),
          };
          ui.colored_label(color, text);

          ui.end_row();
        });
      ui.add_space(20.);

      ui.heading(t!("Dictionary"));
      ui.separator();

      Grid::new("dictionary grid")
        .num_columns(4)
        .min_col_width(150.)
        .spacing([5., 5.])
        .striped(true)
        .show(ui, |ui| {
          ComboBox::from_id_source("languages")
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
                      .pick_language_by_name(self.selected_language.clone())
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

          let (text, color) = match (
            self.dict_metadata.manifest.checksum == self.dict_checksum,
            self.selected_language == "None",
            self.dict_metadata.vec_manifests.len() == 0,
          ) {
            (_, _, true) => (
              format!("✖ {}", t!("dictionary data was not loaded")),
              COLOR_ERROR,
            ),
            (true, false, false) => (format!("✅ {}", t!("up-to-date")), COLOR_UP_TO_DATE),
            (false, false, false) => (format!("⚠ {}", t!("update available")), COLOR_UPDATE_AVAILABLE),
            (_, true, false) => (format!("⚠ {}", t!("choose language")), COLOR_CHOOSE_LANGUAGE),
          };
          ui.colored_label(color, text);
          ui.end_row();
        });
      ui.add_space(20.);

      if (self.hook_metadata.manifest.df == self.bin.checksum
        && self.hook_metadata.manifest.checksum != self.hook_checksum)
        || (self.dict_metadata.manifest.checksum != self.dict_checksum && self.selected_language != "None")
      {
        ui.style_mut().text_styles.insert(
          TextStyle::Button,
          FontId::new(20., eframe::epaint::FontFamily::Proportional),
        );
        ui.vertical_centered(|ui| {
          if self.loading > 0 {
            ui.add(Spinner::new().size(40.));
          } else {
            let button = ui.add_sized([130., 40.], Button::new(t!("Update")));
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
