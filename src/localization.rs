use std::collections::HashMap;

#[static_init::dynamic]
pub static LOCALE: Localization = Localization::new();

macro_rules! t {
  ($l:expr) => {
    LOCALE.get($l)
  };
}
pub(crate) use t;

pub struct Localization {
  map: HashMap<String, String>,
}

impl Localization {
  fn new() -> Self {
    let current_locale = sys_locale::get_locale().unwrap_or("en-US".to_owned());
    let translations = HashMap::<String, &'static str>::from([
      ("en-US".to_owned(), std::include_str!("../locale/en-US.json")),
      ("ru-RU".to_owned(), std::include_str!("../locale/ru-RU.json")),
    ]);

    let translation: HashMap<String, String> = match translations.get(&current_locale) {
      Some(v) => serde_json::from_str(v).unwrap(),
      None => serde_json::from_str(translations.get("en-US").unwrap()).unwrap(),
    };

    Self { map: translation }
  }

  pub fn get(&self, s: &str) -> String {
    self.map.get(s).unwrap_or(&"unknown key".to_owned()).to_owned()
  }
}
