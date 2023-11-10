pub static BOOSTY_ICON: eframe::egui::widgets::ImageSource<'static> =
  eframe::egui::include_image!("../assets/boosty.png");
pub static GITHUB_ICON: eframe::egui::widgets::ImageSource<'static> =
  eframe::egui::include_image!("../assets/github.png");
pub static APP_ICON: &'static [u8; 1980] = include_bytes!("../assets/df.png");

pub static PATH_CACHE_FILE: &'static str = "./dfint-installer.cache";

pub static URL_HOOK_MANIFEST: &'static str =
  "https://raw.githubusercontent.com/dfint/update-metadata/main/metadata/hook.json";
pub static URL_DICT_MANIFEST: &'static str =
  "https://raw.githubusercontent.com/dfint/update-metadata/main/metadata/dict.json";

pub static PATH_DATA: &'static str = "dfint-data";
pub static PATH_CONFIG: &'static str = "dfint-data/config.toml";
pub static PATH_OFFSETS: &'static str = "dfint-data/offsets.toml";
pub static PATH_DICT: &'static str = "dfint-data/dict.csv";
pub static PATH_FONT: &'static str = "dfint-data/curses.png";
pub static PATH_ENCODING: &'static str = "dfint-data/encoding.toml";
