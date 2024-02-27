use std::collections::HashMap;

#[static_init::dynamic]
pub static mut LOCALE: Localization = {
  let locale = sys_locale::get_locale().unwrap_or("en-US".to_owned());
  Localization::new(locale)
};

#[static_init::dynamic]
static TRANSLATIONS: HashMap<String, &'static str> = HashMap::from([
  ("en-US".to_owned(), std::include_str!("../locale/en-US.json")),
  ("ru-RU".to_owned(), std::include_str!("../locale/ru-RU.json")),
]);

macro_rules! t {
  ($l:expr) => {
    LOCALE.read().get($l)
  };
}
pub(crate) use t;

pub struct Localization {
  map: HashMap<String, String>,
  locale: String,
}

impl Localization {
  pub fn new(locale: String) -> Self {
    let translation: HashMap<String, String> = match TRANSLATIONS.get(&locale) {
      Some(v) => serde_json::from_str(v).unwrap(),
      None => serde_json::from_str(TRANSLATIONS.get("en-US").unwrap()).unwrap(),
    };

    Self {
      map: translation,
      locale,
    }
  }

  pub fn get(&self, s: &str) -> String {
    self.map.get(s).unwrap_or(&"unknown key".to_owned()).to_owned()
  }

  pub fn set(&mut self, s: &str) {
    self.locale = s.to_owned();
    self.map = match TRANSLATIONS.get(s) {
      Some(v) => serde_json::from_str(v).unwrap(),
      None => serde_json::from_str(TRANSLATIONS.get("en-US").unwrap()).unwrap(),
    };
  }

  pub fn current_locale(&self) -> String {
    self.locale.clone()
  }

  pub fn locales(&self) -> Vec<String> {
    TRANSLATIONS.keys().cloned().collect()
  }
}
