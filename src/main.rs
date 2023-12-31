#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate serde_derive;

use anyhow::Result;
use constants::APP_ICON;
use eframe::{egui, IconData};

mod app;
mod constants;
mod localization;
mod logic;
mod persistent;
mod state;
mod utils;

fn main() -> Result<(), eframe::Error> {
  env_logger::init();
  let options = eframe::NativeOptions {
    initial_window_size: Some(egui::vec2(720., 450.)),
    resizable: false,
    icon_data: Some(IconData::try_from_png_bytes(APP_ICON).unwrap()),
    ..Default::default()
  };
  eframe::run_native(
    "DF localization installer",
    options,
    Box::new(|cc| {
      egui_extras::install_image_loaders(&cc.egui_ctx);
      Box::<app::App>::default()
    }),
  )
}
