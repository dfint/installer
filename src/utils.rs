use core::fmt;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use exe::{VecPE, PE};
use sysinfo::System;

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

pub async fn download_to_file(url: String, file: PathBuf) -> Result<()> {
  let mut data: Vec<u8> = vec![];
  ureq::get(&url)
    .call()?
    .into_reader()
    .read_to_end(&mut data)?;
  std::fs::write(file, &data)?;
  Ok(())
}

// TODO: make it concurrent
pub async fn batch_download_to_file(items: Vec<(String, PathBuf)>) -> Result<()> {
  for item in items {
    download_to_file(item.0, item.1).await?;
  }
  Ok(())
}
