use anyhow::Result;
use exe::{VecPE, PE};

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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
      valid: false,
      lib_path: std::env::current_dir().expect("Unable to locate current dir"),
      dfhooks_path: std::env::current_dir().expect("Unable to locate current dir"),
    }
  }
}

impl std::fmt::Display for DfBinary {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    std::write!(f, "{}", self.path.as_path().display().to_string())
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

    Self {
      path: path.clone(),
      dir: parent.to_path_buf(),
      checksum: checksum.expect("err check above"),
      os,
      valid: true,
      lib_path: Self::get_lib_path(parent, os, "dfhooks_dfint"),
      dfhooks_path: Self::get_lib_path(parent, os, "dfhooks"),
    }
  }

  fn os(path: &PathBuf) -> OS {
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
}
