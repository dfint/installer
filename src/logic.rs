use core::fmt;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use exe::{VecPE, PE};
use sysinfo::{System, SystemExt};

use crate::constants::*;

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Notification {
  None,
  Error,
  Warning,
  Info,
  Success,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OS {
  None = -1,
  Linux = 0,
  Windows = 1,
}

impl fmt::Display for OS {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      OS::None => std::write!(f, "None"),
      OS::Linux => std::write!(f, "Linux"),
      OS::Windows => std::write!(f, "Windows"),
    }
  }
}

pub fn df_checksum(path: &Option<PathBuf>, os: OS) -> Result<u32> {
  match (path, os) {
    (Some(pathbuf), OS::Windows) => {
      let pefile = VecPE::from_disk_file(pathbuf)?;
      Ok(pefile.get_nt_headers_64()?.file_header.time_date_stamp)
    }
    (Some(pathbuf), OS::Linux) => Ok(crc(pathbuf)?),
    _ => Err(anyhow!("Unknown os").into()),
  }
}

pub fn crc(path: &PathBuf) -> Result<u32> {
  let content = std::fs::read(path)?;
  Ok(crc32fast::hash(&content))
}

pub fn checksum_for_files(vec: Vec<Option<PathBuf>>) -> Result<u32> {
  let mut data: Vec<u8> = vec![];
  for file in vec {
    match file {
      Some(f) => match std::fs::read(f) {
        Ok(mut c) => data.append(&mut c),
        Err(_) => data.push(0),
      },
      None => data.push(0),
    }
  }
  Ok(crc32fast::hash(&data))
}

pub fn local_hook_checksum(df_bin: &Option<PathBuf>, df_dir: &Option<PathBuf>) -> Result<u32> {
  match df_dir {
    Some(pathbuf) => checksum_for_files(vec![
      df_bin.clone(),
      Some(pathbuf.join(PATH_CONFIG)),
      Some(pathbuf.join(PATH_OFFSETS)),
    ]),
    None => Ok(0),
  }
}

pub fn local_dict_checksum(df_dir: &Option<PathBuf>) -> Result<u32> {
  match df_dir {
    Some(pathbuf) => checksum_for_files(vec![
      Some(pathbuf.join(PATH_DICT)),
      Some(pathbuf.join(PATH_FONT)),
      Some(pathbuf.join(PATH_ENCODING)),
    ]),
    None => Ok(0),
  }
}

pub fn scan_df() -> Option<PathBuf> {
  let current = std::env::current_dir().unwrap();
  let pathes = vec![
    current.join("Dwarf Fortress.exe"),
    current.join("dwarfort"),
    PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Dwarf Fortress\\Dwarf Fortress.exe"),
    PathBuf::from("~/.local/share/Steam/steamapps/common/Dwarf Fortress/dwarfort"),
  ];
  pathes.iter().find(|path| path.exists()).cloned()
}

pub fn create_dir_if_not_exist(df_dir: &Option<PathBuf>) -> Result<()> {
  if let Some(pathbuf) = df_dir {
    std::fs::create_dir_all(pathbuf.join(PATH_DATA))?;
  }
  Ok(())
}

pub fn df_dir_by_bin(path: &Option<PathBuf>) -> Option<PathBuf> {
  match path {
    Some(pathbuf) => match pathbuf.as_path().parent() {
      Some(parent) => Some(parent.to_path_buf()),
      _ => None,
    },
    _ => None,
  }
}

pub fn df_os_by_bin(path: &Option<PathBuf>) -> OS {
  match path {
    Some(pathbuf) => {
      let p = pathbuf.as_path();
      if p.exists() && p.file_name() == Some(OsStr::new("Dwarf Fortress.exe")) {
        OS::Windows
      } else if p.exists() && p.file_name() == Some(OsStr::new("dwarfort")) {
        OS::Linux
      } else {
        OS::None
      }
    }
    _ => OS::None,
  }
}

pub fn is_df_bin(path: &Path) -> bool {
  path.exists() && (path.file_name() == Some(OsStr::new("Dwarf Fortress.exe")))
    || path.file_name() == Some(OsStr::new("dwarfort"))
}

pub fn is_dfhack_installed(df_dir: &Option<PathBuf>) -> bool {
  match df_dir {
    Some(path) => {
      (path.join("dfhooks.dll").exists() || path.join("libdfhooks.so").exists()) && path.join("hack/plugins").exists()
    }
    None => false,
  }
}

pub fn get_lib_path(df_dir: &Option<PathBuf>, os: OS) -> Option<PathBuf> {
  match (df_dir.clone(), os, is_dfhack_installed(&df_dir)) {
    (Some(pathbuf), OS::Windows, true) => Some(pathbuf.join("hack/plugins/dfint-hook.plug.dll")),
    (Some(pathbuf), OS::Windows, false) => Some(pathbuf.join("dfhooks.dll")),
    (Some(pathbuf), OS::Linux, true) => Some(pathbuf.join("hack/plugins/dfint-hook.plug.so")),
    (Some(pathbuf), OS::Linux, false) => Some(pathbuf.join("libdfhooks.so")),
    (_, _, _) => None,
  }
}

pub fn is_df_running() -> bool {
  let s = System::new_all();
  for _ in s.processes_by_exact_name("Dwarf Fortress.exe") {
    return true;
  }
  for _ in s.processes_by_exact_name("dwarfort") {
    return true;
  }
  false
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone)]
pub struct HookManifest {
  pub df: u32,
  pub version: u32,
  pub lib: String,
  pub config: String,
  pub offsets: String,
}

pub fn fetch_hook_manifest() -> Result<Vec<HookManifest>> {
  let manifests: Vec<HookManifest> = ureq::get(URL_HOOK_MANIFEST).call()?.into_json()?;
  Ok(manifests)
}

pub fn get_manifest_by_df(df_checksum: u32, manifests: Vec<HookManifest>) -> Option<HookManifest> {
  manifests.iter().find(|item| item.df == df_checksum).cloned()
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone)]
pub struct DictManifest {
  pub language: String,
  pub version: u32,
  pub csv: String,
  pub font: String,
  pub encoding: String,
}

pub fn fetch_dict_manifest() -> Result<Vec<DictManifest>> {
  let manifests: Vec<DictManifest> = ureq::get(URL_DICT_MANIFEST).call()?.into_json()?;
  return Ok(manifests);
}

pub fn get_manifest_by_language(language: String, manifests: Vec<DictManifest>) -> Option<DictManifest> {
  manifests.iter().find(|item| item.language == language).cloned()
}

pub fn download_to_file(url: &str, file: &PathBuf) -> Result<()> {
  let mut data: Vec<u8> = vec![];
  ureq::get(url).call()?.into_reader().read_to_end(&mut data)?;
  std::fs::write(file, &data)?;
  Ok(())
}

pub fn remove_old_data(df_dir: &Option<PathBuf>) {
  if let Some(pathbuf) = df_dir {
    let _ = std::fs::remove_file(pathbuf.join("dfint_launcher.exe"));
    let _ = std::fs::remove_dir_all(pathbuf.join("dfint_data"));
  }
}
