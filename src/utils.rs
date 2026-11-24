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
  let home = std::env::var("HOME").map(PathBuf::from);

  let mut pathes = vec![
    current.join("Dwarf Fortress.exe"),
    current.join("dwarfort"),
    PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Dwarf Fortress\\Dwarf Fortress.exe"),
  ];
  if let Ok(home) = home {
    let steam_game_dir = home.join(".local/share/Steam/steamapps/common/Dwarf Fortress");
    pathes.push(steam_game_dir.join("dwarfort"));
    pathes.push(steam_game_dir.join("Dwarf Fortress.exe"));
  }
  pathes.into_iter().find(|path| path.exists())
}

pub fn is_df_running() -> bool {
  let Ok(system) = System::new_all() else {
    return false;
  };

  system.processes().values().any(|val: &Process| {
    [
      OsStr::new("Dwarf Fortress.exe"),
      OsStr::new("dwarfort"),
      OsStr::new("Dwarf Fortress."),
    ]
    .contains(&val.name())
  })
}
