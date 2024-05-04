pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub const GITHUB_ICON: eframe::egui::widgets::ImageSource<'static> =
  eframe::egui::include_image!("../assets/github.png");
pub const TRANSIFEX_ICON: eframe::egui::widgets::ImageSource<'static> =
  eframe::egui::include_image!("../assets/transifex.png");
pub const APP_ICON: &'static [u8; 1980] = include_bytes!("../assets/df.png");
pub const ORIGINAL_FONT: &'static [u8; 1568] = include_bytes!("../assets/original_font.png");

pub const PATH_CACHE_FILE: &'static str = "./dfint-installer.cache";

pub const URL_HOOK_MANIFEST: &'static str = "https://dfint.github.io/update-data/metadata/hook_v2.json";
pub const URL_DICT_MANIFEST: &'static str = "https://dfint.github.io/update-data/metadata/dict.json";
pub const URL_BUGS: &'static str = "https://github.com/dfint/installer/issues";
pub const URL_TRANSIFEX: &'static str =
  "https://explore.transifex.com/dwarf-fortress-translation/dwarf-fortress-steam/";

pub const PATH_DATA: &'static str = "dfint-data";
pub const PATH_CONFIG: &'static str = "dfint-data/config.toml";
pub const PATH_OFFSETS: &'static str = "dfint-data/offsets.toml";
pub const PATH_DICT: &'static str = "dfint-data/dictionary.csv";
pub const PATH_FONT: &'static str = "data/art/curses_640x300.png";
pub const PATH_ENCODING: &'static str = "dfint-data/encoding.toml";
