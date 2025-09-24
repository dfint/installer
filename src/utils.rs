use std::{ffi::OsStr, path::PathBuf};

use anyhow::Result;
use sysinfo::{Process, System};

pub fn checksum_for_files(vec: Vec<PathBuf>) -> Result<u32> {
  let mut data: Vec<u8> = vec![];
  for file in vec {
    match std::fs::read(file) {
      Ok(mut c) => data.append(&mut c),
      Err(_) => data.push(0),
    }
  }
  Ok(crc32fast::hash(&data))
}

pub fn scan_df() -> Option<PathBuf> {
  let current = std::env::current_dir().unwrap();
  let pathes = [
    current.join("Dwarf Fortress.exe"),
    current.join("dwarfort"),
    PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Dwarf Fortress\\Dwarf Fortress.exe"),
    PathBuf::from("~/.local/share/Steam/steamapps/common/Dwarf Fortress/dwarfort"),
  ];
  pathes.iter().find(|path| path.exists()).cloned()
}

pub async fn is_df_running() -> bool {
  System::new_all().processes().values().any(|val: &Process| {
    [
      OsStr::new("Dwarf Fortress.exe"),
      OsStr::new("dwarfort"),
      OsStr::new("Dwarf Fortress."),
    ]
    .contains(&val.name())
  })
}
