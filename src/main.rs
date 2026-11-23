#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate serde_derive;

use anyhow::Result;
use constants::APP_ICON;
use eframe::egui;

mod app;
mod cache;
mod constants;
mod df_binary;
mod dict_metadata;
mod fetch;
mod hook_metadata;
mod localization;
mod logic;
mod settings;
mod thread_pool;
mod utils;

fn main() -> Result<(), eframe::Error> {
  env_logger::init();
  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([720., 450.])
      .with_resizable(false)
      .with_icon(eframe::icon_data::from_png_bytes(APP_ICON).unwrap()),
    ..Default::default()
  };
  eframe::run_native(
    "DF localization installer",
    options,
    Box::new(|cc| {
      let app = Box::<app::App>::default();
      cc.egui_ctx.set_theme(app.settings.theme_preference());
      egui_extras::install_image_loaders(&cc.egui_ctx);
      Ok(app)
    }),
  )
}
