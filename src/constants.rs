pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ORIGINAL_FONT: &[u8; 1568] = include_bytes!("../assets/original_font.png");
pub const ORIGINAL_FONT_CHECKSUM: u32 = 359_782_074;

pub const PATH_CACHE_FILE: &str = "./dfint-installer.cache";
pub const PATH_SETTINGS_FILE: &str = "./dfint-installer.conf";
pub const PATH_ERROR_FILE: &str = "./dfint-error.log";

pub const BASE_URL: [&str; 2] = [
  "https://dfint.github.io",
  "https://gitverse.ru/api/repos/dfint/data-mirror/raw/branch/master",
];

pub const URL_HOOK_MANIFEST: &str = "/update-data/metadata/hook_v3.json";
pub const URL_DICT_MANIFEST: &str = "/update-data/metadata/dict_v3.json";
pub const URL_BUGS: &str = "https://github.com/dfint/installer/issues";
pub const URL_TRANSIFEX: &str = "https://explore.transifex.com/dwarf-fortress-translation/dwarf-fortress-steam/";

pub const PATH_DATA: &str = "dfint-data";
pub const PATH_CONFIG: &str = "dfint-data/config.toml";
pub const PATH_OFFSETS: &str = "dfint-data/offsets.toml";
pub const PATH_DICT: &str = "dfint-data/dictionary.csv";
pub const PATH_FONT: &str = "data/art/curses_640x300.png";
pub const PATH_ENCODING: &str = "dfint-data/encoding.toml";
