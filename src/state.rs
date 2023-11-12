use crate::logic::{DictManifest, HookManifest, Notification};

macro_rules! read {
  ($l:ident) => {
    STATE.read().$l
  };
}
pub(crate) use read;

macro_rules! write {
  ($l:ident, $v:expr) => {
    STATE.write().$l = $v;
  };
}
pub(crate) use write;

pub struct State {
  pub hook_manifest: HookManifest,
  pub vec_hook_manifests: Vec<HookManifest>,
  pub dict_manifest: DictManifest,
  pub vec_dict_manifests: Vec<DictManifest>,
  pub notify: (Notification, String),
  pub recalculate_hook_checksum: bool,
  pub recalculate_dict_checksum: bool,
  pub loading: u8,
}

#[static_init::dynamic]
pub static mut STATE: State = State {
  hook_manifest: HookManifest {
    df: 0,
    version: 0,
    lib: "".into(),
    config: "".into(),
    offsets: "".into(),
  },
  vec_hook_manifests: vec![],
  dict_manifest: DictManifest {
    language: "-".into(),
    version: 0,
    csv: "".into(),
    font: "".into(),
    encoding: "".into(),
  },
  vec_dict_manifests: vec![],
  notify: (Notification::None, "".into()),
  recalculate_hook_checksum: false,
  recalculate_dict_checksum: false,
  loading: 0,
};
