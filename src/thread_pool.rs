use async_std::task;
use futures::FutureExt;
use std::{
  future::Future,
  sync::mpsc::{channel, Receiver, Sender, TryIter},
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

  pub fn execute<O>(
    &self,
    task: impl Future<Output = O> + Send + 'static,
    message: impl FnOnce(O) -> T + Send + 'static,
  ) {
    let tx = self.tx.clone();
    task::spawn(async move {
      let result = task.map(message).await;
      tx.send(result).unwrap();
    });
  }

  pub fn poll(&self) -> TryIter<'_, T> {
    self.rx.try_iter()
  }
}
