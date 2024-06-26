use crate::codec::{Deserializer, Serializer};
use bytes::BytesMut;
use futures::stream::Stream;
use futures::{prelude::*, task::*};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::{error::Error, io, pin::Pin};

// Wraps a websocket S into a tarpc transport with the given codec.
#[pin_project]
pub struct WsTransport<S, Item, SinkItem, Codec> {
    #[pin]
    inner: S,
    #[pin]
    codec: Codec,
    item: PhantomData<(Item, SinkItem)>,
}

impl<S, Item, SinkItem, Codec> WsTransport<S, Item, SinkItem, Codec> {
    pub fn new(inner: S, codec: Codec) -> Self {
        Self {
            inner,
            codec,
            item: PhantomData,
        }
    }

    pub fn get_ref(&self) -> &S {
        &self.inner
    }
}

impl<S, Item, SinkItem, Codec> Stream for WsTransport<S, Item, SinkItem, Codec>
where
    S: Stream,
    Item: for<'a> Deserialize<'a>,
    <S as Stream>::Item: AsRef<[u8]>,
    Codec: Deserializer<Item>,
    <Codec as Deserializer<Item>>::Error: Into<Box<dyn Error + Send + Sync>>,
{
    type Item = std::io::Result<Item>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::io::Result<Item>>> {
        self.as_mut()
            .project()
            .inner
            .poll_next(cx)
            .map(|res| match res {
                Some(item) => Some(
                    self.as_mut()
                        .project()
                        .codec
                        .deserialize(&BytesMut::from(item.as_ref()))
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
                ),
                None => None,
            })
    }
}

impl<S, Item, SinkItem, Codec> Sink<SinkItem> for WsTransport<S, Item, SinkItem, Codec>
where
    S: Stream + Sink<<S as Stream>::Item>,
    <S as Stream>::Item: From<Vec<u8>>,
    SinkItem: Serialize,
    S::Error: Into<Box<dyn Error + Send + Sync>>,
    Codec: Serializer<SinkItem>,
    <Codec as Serializer<SinkItem>>::Error: Into<Box<dyn Error + Send + Sync>>,
{
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .poll_ready(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn start_send(mut self: Pin<&mut Self>, item: SinkItem) -> io::Result<()> {
        let res = self.as_mut().project().codec.serialize(&item);
        let bytes = res.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        // TODO: we have to convert the serialized bytes into a websocket message that can be used by
        // inner.start_send, which is redundant and may introduce a copy. Check if we can send the bytes directly
        let msg = <S as Stream>::Item::from(Vec::from(bytes));
        self.as_mut()
            .project()
            .inner
            .start_send(msg)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .poll_flush(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .poll_close(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
