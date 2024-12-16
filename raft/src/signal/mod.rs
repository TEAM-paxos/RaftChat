use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::sync::oneshot;

pub struct Sender(oneshot::Sender<()>);
pub struct Receiver(oneshot::Receiver<()>);

impl Sender {
    pub fn signal(self) {
        let _ = self.0.send(());
    }
}

impl Future for Receiver {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn channel() -> (Sender, Receiver) {
    let (tx, rx) = oneshot::channel();
    (Sender(tx), Receiver(rx))
}
