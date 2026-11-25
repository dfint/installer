use crate::{
  df_binary::{DfBinary, OS},
  dict_metadata::DictMetadata,
  hook_metadata::HookMetadata,
  localization::{LOCALE, t},
  logic::Message,
  settings::Settings,
  thread_pool::ThreadPool,
};

#[derive(PartialEq)]
pub enum State {
  Loading,
  Idle,
}

pub struct App {
  pub settings: Settings,
  pub pool: ThreadPool<Message>,
  pub notice: Option<String>,
  pub process_checked: bool,
  pub delete_old_data_show: bool,
  pub loading: u8,
  pub df_running: bool,
  pub selected_language: String,
  pub ui_locale: String,
  pub hook_checksum: u32,
  pub dict_checksum: u32,
  pub hook_metadata_loading: bool,
  pub dict_metadata_loading: bool,
  pub hook_metadata: HookMetadata,
  pub dict_metadata: DictMetadata,
  pub bin: DfBinary,
  pub state: State,
}

impl Default for App {
  fn default() -> Self {
    Self {
      settings: Settings::load(),
      pool: ThreadPool::new(),
      notice: None,
      process_checked: false,
      delete_old_data_show: false,
      loading: 0,
      df_running: false,
      selected_language: "None".to_string(),
      ui_locale: LOCALE.read().current_locale(),
      hook_checksum: 0,
      dict_checksum: 0,
      hook_metadata_loading: false,
      dict_metadata_loading: false,
      hook_metadata: HookMetadata::default(),
      dict_metadata: DictMetadata::default(),
      bin: DfBinary::default(),
      state: State::Loading,
    }
  }
}

#[derive(Clone, Copy)]
pub enum Status {
  Loading,
  Error,
  Current,
  Attention,
}

impl Status {
  #[cfg(target_os = "linux")]
  pub fn symbol(self) -> &'static str {
    match self {
      Self::Loading => "○",
      Self::Error => "✕",
      Self::Current => "✓",
      Self::Attention => "⚠",
    }
  }

  pub fn color(self, dark_background: bool) -> (u8, u8, u8) {
    match (self, dark_background) {
      (Self::Loading, false) => (29, 92, 173),
      (Self::Error, false) => (177, 38, 38),
      (Self::Current, false) => (28, 112, 48),
      (Self::Attention, false) => (137, 94, 0),
      (Self::Loading, true) => (112, 173, 255),
      (Self::Error, true) => (255, 115, 115),
      (Self::Current, true) => (106, 192, 110),
      (Self::Attention, true) => (230, 184, 0),
    }
  }
}

pub struct ComponentStatus {
  pub local: String,
  pub available: String,
  pub text: String,
  pub status: Status,
}

pub struct View {
  pub path: String,
  pub version: String,
  pub platform: OS,
  pub distribution: &'static str,
  pub checksum: String,
  pub hook: ComponentStatus,
  pub dictionary: ComponentStatus,
  pub languages: Vec<String>,
  pub action: String,
  pub can_update: bool,
  pub has_localization: bool,
  pub ready: bool,
  pub busy: bool,
}

impl App {
  pub fn ready(&self) -> bool {
    self.state == State::Idle && self.process_checked && !self.df_running
  }

  pub fn select_language(&mut self, language: String) {
    self.dict_metadata.pick_language_by_name(language.clone());
    self.selected_language = language;
  }

  pub fn select_locale(&mut self, locale: String) {
    LOCALE.write().set(&locale);
    self.ui_locale = locale;
  }

  pub fn hook_update_available(&self) -> bool {
    let hook = &self.hook_metadata.manifest;
    hook.checksum != 0 && hook.df == self.bin.checksum && hook.checksum != self.hook_checksum
  }

  pub fn dictionary_update_available(&self) -> bool {
    let dict = &self.dict_metadata.manifest;
    dict.checksum != 0 && dict.checksum != self.dict_checksum && self.selected_language != "None"
  }

  pub fn view(&self) -> View {
    let ready = self.ready() && self.bin.valid;
    let busy = self.loading > 0;
    let hook = &self.hook_metadata.manifest;
    let dict = &self.dict_metadata.manifest;
    let hook_status = if !ready || self.hook_metadata_loading {
      (Status::Loading, t!("loading data"))
    } else if self.hook_metadata.vec_manifests.is_empty() && hook.checksum == 0 {
      (Status::Error, t!("hook data was not loaded"))
    } else if hook.df != self.bin.checksum {
      (Status::Error, t!("this DF version is not supported"))
    } else if hook.checksum == self.hook_checksum {
      (Status::Current, t!("up-to-date"))
    } else {
      (Status::Attention, t!("update available"))
    };
    let dict_status = if !ready || self.dict_metadata_loading {
      (Status::Loading, t!("loading data"))
    } else if self.dict_metadata.vec_manifests.is_empty() {
      (Status::Error, t!("dictionary data was not loaded"))
    } else if self.selected_language == "None" {
      (Status::Attention, t!("choose language"))
    } else if dict.checksum == self.dict_checksum {
      (Status::Current, t!("up-to-date"))
    } else {
      (Status::Attention, t!("update available"))
    };
    let has_localization = self.bin.valid && self.any_localization_files_present();
    View {
      path: self.bin.to_string(),
      version: format!("{} |", self.bin.version),
      platform: self.bin.os,
      distribution: if self.bin.steam { "steam" } else { "non-steam" },
      checksum: format!("{:x}", self.bin.checksum),
      hook: component_status(self.hook_checksum, hook.checksum, hook_status),
      dictionary: component_status(self.dict_checksum, dict.checksum, dict_status),
      languages: self
        .dict_metadata
        .vec_manifests
        .iter()
        .map(|item| item.language.clone())
        .collect(),
      action: if has_localization { t!("Update") } else { t!("Install") },
      can_update: ready && !busy && (self.hook_update_available() || self.dictionary_update_available()),
      has_localization,
      ready,
      busy,
    }
  }
}

fn component_status(local: u32, available: u32, (status, text): (Status, String)) -> ComponentStatus {
  ComponentStatus {
    local: local.to_string(),
    available: if available == 0 {
      "?".into()
    } else {
      available.to_string()
    },
    text,
    status,
  }
}

pub fn locales(current: &str) -> Vec<String> {
  let mut locales = LOCALE.read().locales();
  if !locales.iter().any(|locale| locale == current) {
    locales.push(current.into());
  }
  locales.sort();
  locales
}

pub fn running_warning() -> String {
  t!("Dwarf Fortress is running. Close it before using the installer.")
}

pub fn old_data_warning() -> String {
  t!("Old version of translation files has been detected. It's better to delete them to avoid conflicts. Delete?")
}
