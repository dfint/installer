use std::{
  sync::mpsc::{Receiver, Sender, TryIter, channel},
  thread,
};

pub struct ThreadPool<T> {
  tx: Sender<T>,
  rx: Receiver<T>,
}

impl<T: Send + 'static> ThreadPool<T> {
  pub fn new() -> Self {
    let (tx, rx) = channel::<T>();
    ThreadPool { tx, rx }
  }

  pub fn execute<O>(&self, task: impl FnOnce() -> O + Send + 'static, message: impl FnOnce(O) -> T + Send + 'static) {
    let tx = self.tx.clone();
    thread::spawn(move || {
      let _ = tx.send(message(task()));
    });
  }

  pub fn poll(&self) -> TryIter<'_, T> {
    self.rx.try_iter()
  }
}
