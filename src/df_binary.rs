use anyhow::Result;
use exe::{PE, VecPE};

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
      OS::Linux => std::write!(f, "Linux"),
      OS::Windows => std::write!(f, "Windows"),
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

  fn version_comparing_key(version: &str) -> Option<(u32, u32, u32)> {
    let (major, rest) = version.split_once('.')?;
    let (minor, beta) = match rest.split_once('-') {
      Some((minor, suffix)) => {
        let digit = suffix.find(|c: char| c.is_ascii_digit())?;
        if digit == 0 {
          return None;
        }
        (minor, suffix[digit..].parse().ok()?)
      }
      None => (rest, MAX_BETA),
    };
    if !major.bytes().all(|b| b.is_ascii_digit()) || !minor.bytes().all(|b| b.is_ascii_digit()) {
      return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?, beta))
  }

  pub fn detect_df_version(data: &[u8]) -> Option<String> {
    let mut boundaries = memchr::memchr_iter(0, data);
    let mut start = boundaries.next()?;
    let mut best = None;
    for end in boundaries {
      let bytes = &data[start + 1..end];
      start = end;
      if !bytes.first().is_some_and(u8::is_ascii_digit) {
        continue;
      }
      let Ok(version) = std::str::from_utf8(bytes) else {
        continue;
      };
      if let Some(key) = Self::version_comparing_key(version)
        && best.is_none_or(|(previous, _)| key >= previous)
      {
        best = Some((key, version));
      }
    }
    best.map(|(_, version)| version.to_owned())
  }

  pub fn detect_steam_api(data: &[u8]) -> bool {
    memchr::memmem::find(data, b"SteamAPI_Init").is_some()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_latest_version_in_binary_data() {
    let data =
      b"99.99\x00\xff\x000.55.1\x0055.02-beta2\x0055.02-beta10\x0055.02\x0055.03-beta99x\x004294967296.01\x0066.99";
    assert_eq!(DfBinary::detect_df_version(data).as_deref(), Some("55.02"));
    assert_eq!(
      DfBinary::detect_df_version(b"\x0055.02-beta2\x0055.02-beta10\x00").as_deref(),
      Some("55.02-beta10")
    );
  }

  #[test]
  fn bin() {
    let bin = DfBinary::new(PathBuf::from("D:\\Downloads\\Dwarf Fortress.exe"));
    println!("steam: {:?}, version: {:?}", bin.steam, bin.version);
  }
}
