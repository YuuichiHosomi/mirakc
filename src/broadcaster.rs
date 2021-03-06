use std::fmt;
use std::io;
use std::pin::Pin;
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix::dev::{MessageResponse, ResponseChannel};
use bytes::Bytes;
use humantime;
use log;
use tokio::io::AsyncRead;
use tokio::sync::mpsc;

use crate::chunk_stream::ChunkStream;
use crate::tuner::TunerSessionId as BroadcasterId;
use crate::tuner::TunerSubscriptionId as SubscriberId;

struct Subscriber {
    id: SubscriberId,
    sender: mpsc::Sender<Bytes>,
}

pub struct Broadcaster {
    id: BroadcasterId,
    subscribers: Vec<Subscriber>,
    time_limit: Duration,
    last_received: Instant,
}

impl Broadcaster {
    // large enough for 10 sec buffering.
    const MAX_CHUNKS: usize = 1000;

    // 32 KiB, large enough for 10 ms buffering.
    const CHUNK_SIZE: usize = 4096 * 8;

    pub fn new<R>(
        id: BroadcasterId,
        source: R,
        time_limit: u64,
        ctx: &mut Context<Self>,
    ) -> Self
    where
        R: AsyncRead + Unpin + 'static,
    {
        let stream = ChunkStream::new(source, Self::CHUNK_SIZE);
        let _ = Self::add_stream(stream, ctx);
        Self {
            id,
            subscribers: Vec::new(),
            time_limit: Duration::from_millis(time_limit),
            last_received: Instant::now(),
        }
    }

    fn subscribe(&mut self, id: SubscriberId) -> BroadcasterStream {
        let (sender, receiver) = mpsc::channel(Self::MAX_CHUNKS);
        self.subscribers.push(Subscriber { id, sender });
        BroadcasterStream::new(receiver)
    }

    fn unsubscribe(&mut self, id: SubscriberId) {
        // Log warning message if the user haven't subscribed.
        self.subscribers.retain(|subscriber| subscriber.id != id);
    }

    fn broadcast(&mut self, chunk: Bytes) {
        for subscriber in self.subscribers.iter_mut() {
            let chunk_size = chunk.len();
            match subscriber.sender.try_send(chunk.clone()) {
                Ok(_) => {
                    log::trace!("{}: Sent a chunk of {} bytes to {}",
                                self.id, chunk_size, subscriber.id);
                },
                Err(mpsc::error::TrySendError::Full(_)) => {
                    log::warn!("{}: No space for {}, drop the chunk",
                               self.id, subscriber.id);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    log::debug!("{}: Closed by {}, wait for unsubscribe",
                                self.id, subscriber.id);
                }
            }
        }

        self.last_received = Instant::now();
    }

    fn check_timeout(&mut self, ctx: &mut Context<Self>) {
        let elapsed = self.last_received.elapsed();
        if  elapsed > self.time_limit {
            log::error!("{}: No packet from the tuner for {}, stop",
                        self.id, humantime::format_duration(elapsed));
            ctx.stop();
        }
    }
}

impl Actor for Broadcaster {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(self.time_limit, Self::check_timeout);
        log::debug!("{}: Started", self.id);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        log::debug!("{}: Stopped", self.id);
    }
}

// subscribe

pub struct SubscribeMessage {
    pub id: SubscriberId
}

impl fmt::Display for SubscribeMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Subscribe with {}", self.id)
    }
}

impl Message for SubscribeMessage {
    type Result = BroadcasterStream;
}

impl<A, M> MessageResponse<A, M> for BroadcasterStream
where
    A: Actor,
    M: Message<Result = BroadcasterStream>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

impl Handler<SubscribeMessage> for Broadcaster {
    type Result = BroadcasterStream;

    fn handle(
        &mut self,
        msg: SubscribeMessage,
        _: &mut Self::Context
    ) -> Self::Result {
        log::debug!("{}", msg);
        self.subscribe(msg.id)
    }
}

// unsubscribe

pub struct UnsubscribeMessage {
    pub id: SubscriberId
}

impl fmt::Display for UnsubscribeMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsubscribe with {}", self.id)
    }
}

impl Message for UnsubscribeMessage {
    type Result = ();
}

impl Handler<UnsubscribeMessage> for Broadcaster {
    type Result = ();

    fn handle(
        &mut self,
        msg: UnsubscribeMessage,
        _: &mut Self::Context
    ) -> Self::Result {
        log::debug!("{}", msg);
        self.unsubscribe(msg.id)
    }
}

// stream handler

impl StreamHandler<io::Result<Bytes>> for Broadcaster {
    fn handle(&mut self, chunk: io::Result<Bytes>, ctx: &mut Context<Self>) {
        match chunk {
            Ok(chunk) => {
                self.broadcast(chunk);
            }
            Err(err) => {
                log::error!("{}: Error, stop: {}", self.id, err);
                ctx.stop();
            }
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        log::debug!("{}: EOS reached, stop", self.id);
        ctx.stop();
    }
}

// stream

pub struct BroadcasterStream(mpsc::Receiver<Bytes>);

impl BroadcasterStream {
    fn new(rx: mpsc::Receiver<Bytes>) -> Self {
        Self(rx)
    }

    #[cfg(test)]
    pub fn new_for_test() -> (mpsc::Sender<Bytes>, Self) {
        let (tx, rx) = mpsc::channel(10);
        (tx, BroadcasterStream::new(rx))
    }
}

impl Stream for BroadcasterStream {
    type Item = io::Result<Bytes>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.0)
            .poll_next(cx)
            .map(|item| item.map(|chunk| Ok(chunk)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp;
    use std::pin::Pin;
    use std::task::{Poll, Context};
    use bytes::Buf;
    use tokio::stream::StreamExt;
    use tokio::sync::mpsc;

    #[actix_rt::test]
    async fn test_broadcast() {
        let (mut tx, rx) = mpsc::channel(1);

        let broadcaster = Broadcaster::create(|ctx| {
            Broadcaster::new(Default::default(), DataSource(rx), 1000, ctx)
        });

        let mut stream1 = broadcaster.send(SubscribeMessage {
            id: SubscriberId::new(Default::default(), 1)
        }).await.unwrap();

        let mut stream2 = broadcaster.send(SubscribeMessage {
            id: SubscriberId::new(Default::default(), 2)
        }).await.unwrap();

        let _ = tx.send(Bytes::from("hello")).await;

        let chunk = stream1.next().await;
        assert!(chunk.is_some());

        let chunk = stream2.next().await;
        assert!(chunk.is_some());
    }

    #[actix_rt::test]
    async fn test_unsubscribe() {
        let (mut tx, rx) = mpsc::channel(1);

        let broadcaster = Broadcaster::create(|ctx| {
            Broadcaster::new(Default::default(), DataSource(rx), 1000, ctx)
        });

        let mut stream1 = broadcaster.send(SubscribeMessage {
            id: SubscriberId::new(Default::default(), 1)
        }).await.unwrap();

        let mut stream2 = broadcaster.send(SubscribeMessage {
            id: SubscriberId::new(Default::default(), 2)
        }).await.unwrap();

        broadcaster.send(UnsubscribeMessage {
            id: SubscriberId::new(Default::default(), 1)
        }).await.unwrap();

        let _ = tx.send(Bytes::from("hello")).await;

        let chunk = stream1.next().await;
        assert!(chunk.is_none());

        let chunk = stream2.next().await;
        assert!(chunk.is_some());
    }

    #[actix_rt::test]
    async fn test_timeout() {
        let (mut tx, rx) = mpsc::channel(1);

        let broadcaster = Broadcaster::create(|ctx| {
            Broadcaster::new(Default::default(), DataSource(rx), 50, ctx)
        });

        let mut stream1 = broadcaster.send(SubscribeMessage {
            id: SubscriberId::new(Default::default(), 1)
        }).await.unwrap();

        let _ = tx.send(Bytes::from("hello")).await;

        let chunk = stream1.next().await;
        assert!(chunk.is_some());

        while broadcaster.connected() {
            // Yield in order to process messages on the Broadcaster.
            tokio::task::yield_now().await;
        }

        let _ = tx.send(Bytes::from("hello")).await;

        let chunk = stream1.next().await;
        assert!(chunk.is_none());
    }

    // we can use `futures::stream::repeat(1)` as data source in tests once
    // actix/actix/pull/363 is release.
    struct DataSource(mpsc::Receiver<Bytes>);

    impl AsyncRead for DataSource {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8]
        ) -> Poll<io::Result<usize>> {
            match Pin::new(&mut self.0).poll_next(cx) {
                Poll::Ready(Some(mut chunk)) => {
                    let len = cmp::min(chunk.len(), buf.len());
                    chunk.copy_to_slice(&mut buf[..len]);
                    Poll::Ready(Ok(len))
                }
                Poll::Ready(None) => Poll::Ready(Ok(0)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
