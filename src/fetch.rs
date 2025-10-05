use anyhow::Result;
use std::{
  path::PathBuf,
  sync::atomic::{AtomicUsize, Ordering},
};

use crate::{constants::BASE_URL, fetch};

static BASE_URL_INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn get_base_url() -> &'static str {
  let index = BASE_URL_INDEX.load(Ordering::Relaxed);
  BASE_URL[index.min(BASE_URL.len() - 1)]
}

pub fn switch_to_next_base_url() {
  BASE_URL_INDEX
    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |index| {
      let max_index = BASE_URL.len() - 1;
      if index < max_index { Some(index + 1) } else { None }
    })
    .ok();
}

pub fn fetch_json<T: for<'de> serde::Deserialize<'de>>(path: &str) -> Result<T> {
  let base_url = get_base_url();
  let url = format!("{}{}", base_url, path);

  match ureq::get(&url).call() {
    Ok(res) => Ok(res.into_json().unwrap()),
    Err(e) => {
      if get_base_url() == base_url {
        switch_to_next_base_url();
        if get_base_url() != base_url {
          return fetch_json(path);
        }
      } else {
        return fetch_json(path);
      }

      Err(anyhow::Error::from(e))
    }
  }
}

pub fn fetch_bytes(path: &str) -> Result<Vec<u8>> {
  let base_url = get_base_url();
  let url = format!("{}{}", base_url, path);

  match ureq::get(&url).call() {
    Ok(res) => {
      let mut bytes = Vec::new();
      res.into_reader().read_to_end(&mut bytes)?;
      Ok(bytes)
    }
    Err(e) => {
      if get_base_url() == base_url {
        switch_to_next_base_url();
        if get_base_url() != base_url {
          return fetch_bytes(path);
        }
      } else {
        return fetch_bytes(path);
      }

      Err(anyhow::Error::from(e))
    }
  }
}

pub async fn download_to_file(url: &str, file: &PathBuf) -> Result<()> {
  let data: Vec<u8> = fetch::fetch_bytes(url)?;
  std::fs::write(file, &data)?;
  Ok(())
}

pub async fn batch_download_to_file(items: Vec<(String, PathBuf)>) -> Result<()> {
  let result = futures::future::join_all(items.iter().map(|(url, file)| download_to_file(url, file))).await;
  for item in result {
    item?;
  }
  Ok(())
}
