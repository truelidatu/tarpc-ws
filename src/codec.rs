use bytes::{Bytes, BytesMut};
use std::pin::Pin;
pub trait Serializer<T> {
    type Error;

    /// Serializes `item` into a new buffer
    ///
    /// The serialization format is specific to the various implementations of
    /// `Serializer`. If the serialization is successful, a buffer containing
    /// the serialized item is returned. If the serialization is unsuccessful,
    /// an error is returned.
    ///
    /// Implementations of this function should not mutate `item` via any sort
    /// of internal mutability strategy.
    ///
    /// See the trait level docs for more detail.
    fn serialize(self: Pin<&mut Self>, item: &T) -> Result<Bytes, Self::Error>;
}

pub trait Deserializer<T> {
    type Error;

    /// Deserializes a value from `buf`
    ///
    /// The serialization format is specific to the various implementations of
    /// `Deserializer`. If the deserialization is successful, the value is
    /// returned. If the deserialization is unsuccessful, an error is returned.
    ///
    /// See the trait level docs for more detail.
    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<T, Self::Error>;
}

pub mod bitcode {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::{marker::PhantomData, pin::Pin};
    pub struct Bitcode<Item, SinkItem> {
        ghost: PhantomData<(Item, SinkItem)>,
    }

    impl<Item, SinkItem> Default for Bitcode<Item, SinkItem> {
        fn default() -> Self {
            Bitcode { ghost: PhantomData }
        }
    }

    pub type SymmetricalBitcode<T> = Bitcode<T, T>;

    impl<Item, SinkItem> Deserializer<Item> for Bitcode<Item, SinkItem>
    where
        for<'a> Item: Deserialize<'a>,
    {
        type Error = std::io::Error;

        fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Item, Self::Error> {
            ::bitcode::deserialize(src)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }

    impl<Item, SinkItem> Serializer<SinkItem> for Bitcode<Item, SinkItem>
    where
        SinkItem: Serialize,
    {
        type Error = std::io::Error;

        fn serialize(self: Pin<&mut Self>, item: &SinkItem) -> Result<Bytes, Self::Error> {
            ::bitcode::serialize(item)
                .map(|v| Bytes::from(v))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }
}
