use anyhow::Result;
use bytes::Bytes;
use futures::future::BoxFuture;
#[allow(unused_imports)]
use value::*;

#[macro_use]
extern crate anyhow;

pub mod handshake;
pub mod transmission;
pub mod server;
#[allow(non_camel_case_types)]
pub mod value;

pub(crate) trait NBDSerailize {
    fn to_nbd_bytes(self) -> Bytes;
}

pub(crate) trait NBDDeserailize<'a, T> {
    fn from_stream(stream: &'a mut T) -> BoxFuture<'a, Result<Self>>
    where
        Self: Sized;
}
