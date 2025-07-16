use anyhow::Result;
use exe::{PE, VecPE};

use regex::bytes::{Captures, Regex};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const MAX_BETA: u32 = 10_000;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OS {
  Linux = 0,
  Windows = 1,
}

impl std::fmt::Display for OS {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      OS::Linux => std::write!(f, "🐧 Linux"),
      OS::Windows => std::write!(f, " Windows"),
    }
  }
}

pub struct DfBinary {
  pub path: PathBuf,
  pub dir: PathBuf,
  pub checksum: u32,
  pub os: OS,
  pub version: String,
  pub steam: bool,
  pub valid: bool,
  pub lib_path: PathBuf,
  pub dfhooks_path: PathBuf,
}

impl Default for DfBinary {
  fn default() -> Self {
    Self {
      path: std::env::current_dir().expect("Unable to locate current dir"),
      dir: std::env::current_dir().expect("Unable to locate current dir"),
      checksum: 0,
      os: OS::Windows,
      version: String::from("Unknown"),
      steam: false,
      valid: false,
      lib_path: std::env::current_dir().expect("Unable to locate current dir"),
      dfhooks_path: std::env::current_dir().expect("Unable to locate current dir"),
    }
  }
}

impl std::fmt::Display for DfBinary {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    std::write!(f, "{}", self.path.as_path().display())
  }
}

impl DfBinary {
  pub fn new(path: PathBuf) -> Self {
    if !path.exists()
      || !(path.file_name() == Some(OsStr::new("Dwarf Fortress.exe"))
        || path.file_name() == Some(OsStr::new("dwarfort")))
    {
      return Self::default();
    }

    let os = Self::os(&path);
    let checksum = Self::checksum(&path, os);
    if checksum.is_err() {
      return Self::default();
    }
    let parent = path.parent().expect("Unable to get parent dir");

    let data = std::fs::read(&path).expect("Unable to read file");
    let version = Self::detect_df_version(&data);
    let steam_api = Self::detect_steam_api(&data);

    Self {
      path: path.clone(),
      dir: parent.to_path_buf(),
      checksum: checksum.expect("err check above"),
      os,
      version: version.unwrap_or_default(),
      steam: steam_api,
      valid: true,
      lib_path: Self::get_lib_path(parent, os, "dfhooks_dfint"),
      dfhooks_path: Self::get_lib_path(parent, os, "dfhooks"),
    }
  }

  fn os(path: &Path) -> OS {
    if path.file_name() == Some(OsStr::new("Dwarf Fortress.exe")) {
      OS::Windows
    } else {
      OS::Linux
    }
  }

  fn checksum(path: &Path, os: OS) -> Result<u32> {
    match os {
      OS::Windows => {
        let pefile = VecPE::from_disk_file(path)?;
        Ok(pefile.get_nt_headers_64()?.file_header.time_date_stamp)
      }
      OS::Linux => {
        let content = std::fs::read(path)?;
        Ok(crc32fast::hash(&content))
      }
    }
  }

  fn get_lib_path(dir: &Path, os: OS, name: &str) -> PathBuf {
    match os {
      OS::Windows => dir.join(format!("{name}.dll")),
      OS::Linux => dir.join(format!("lib{name}.so")),
    }
  }

  fn version_comparing_key(caps: &Captures) -> (u32, u32, u32) {
    let parse_group = |idx: usize| -> u32 {
      let m = caps.get(idx).unwrap().as_bytes();
      std::str::from_utf8(m).unwrap().parse().unwrap()
    };

    let major = parse_group(2);
    let minor = parse_group(3);

    let beta = caps
      .get(6)
      .map(|m| std::str::from_utf8(m.as_bytes()).unwrap().parse().unwrap())
      .unwrap_or(MAX_BETA);

    (major, minor, beta)
  }

  pub fn detect_df_version(data: &[u8]) -> Option<String> {
    let pattern = Regex::new(r"\x00((\d+)\.(\d+)(-([^\d]+)(\d*))?)\x00").unwrap();
    let best_match = pattern
      .captures_iter(data)
      .max_by_key(|caps| DfBinary::version_comparing_key(caps))?;

    let version_bytes = best_match.get(1)?.as_bytes();
    String::from_utf8(version_bytes.to_vec()).ok()
  }

  pub fn detect_steam_api(data: &[u8]) -> bool {
    let pattern = Regex::new(r"SteamAPI_Init").unwrap();
    pattern.is_match(data)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bin() {
    let bin = DfBinary::new(PathBuf::from("D:\\Downloads\\Dwarf Fortress.exe"));
    println!("steam: {:?}, version: {:?}", bin.steam, bin.version);
  }
}
