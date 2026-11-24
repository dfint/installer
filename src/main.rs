#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate serde_derive;

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

#[cfg(target_os = "linux")]
#[path = "gtk_ui.rs"]
mod ui;
#[cfg(windows)]
#[path = "windows_ui.rs"]
mod ui;

fn main() -> anyhow::Result<()> {
  env_logger::init();
  ui::run()
}
