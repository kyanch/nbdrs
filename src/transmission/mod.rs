use anyhow::Ok;
use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    CommandFlags, ErrorType, NBDDeserailize, NBDSerailize, RequestType, NBD_REQUEST_MAGIC,
    NBD_SIMPLE_REPLY_MAGIC,
};

#[derive(Clone, Debug)]
pub struct RequestMsg {
    pub cmd_flags: CommandFlags,
    pub r#type: RequestType,
    pub handle: u64,
    pub offset: u64,
    pub length: u32,
    pub data: Option<Bytes>,
}

impl<'a, T: AsyncRead + Unpin + Send> NBDDeserailize<'a, T> for RequestMsg {
    fn from_stream(
        stream: &'a mut T,
    ) -> futures::prelude::future::BoxFuture<'a, anyhow::Result<Self>>
    where
        Self: Sized,
    {
        Box::pin(async {
            let magic = stream.read_u32().await?;
            if magic != NBD_REQUEST_MAGIC {
                return Err(anyhow!(
                    "The magic {:x} is not match NBD_REQUEST_MAGIC",
                    magic
                ));
            }
            let cmd_flags = stream.read_u16().await?;
            let r#type = stream.read_u16().await?.try_into()?;
            let handle = stream.read_u64().await?;
            let offset = stream.read_u64().await?;
            let length = stream.read_u32().await?;

            if let RequestType::NBD_CMD_WRITE = r#type {
                let mut buf = BytesMut::new();
                buf.resize(length as usize, 0);
                stream.read_exact(&mut buf).await?;
                return Ok(RequestMsg {
                    cmd_flags,
                    r#type,
                    handle,
                    offset,
                    length,
                    data: Some(buf.freeze()),
                });
            }
            Ok(RequestMsg {
                cmd_flags,
                r#type,
                handle,
                offset,
                length,
                data: None,
            })
        })
    }
}

pub struct SimpleReplyMsg {
    error: ErrorType,
    handle: u64,
    data: Option<Bytes>,
}
impl SimpleReplyMsg {
    pub fn new(req: &RequestMsg) -> Self {
        SimpleReplyMsg {
            error: ErrorType::NO_ERROR,
            handle: req.handle,
            data: None,
        }
    }
    pub fn with_err(req: &RequestMsg, err: ErrorType) -> Self {
        SimpleReplyMsg {
            error: err,
            handle: req.handle,
            data: None,
        }
    }
    pub fn with_data(req: &RequestMsg, data: Bytes) -> Self {
        SimpleReplyMsg {
            error: ErrorType::NO_ERROR,
            handle: req.handle,
            data: Some(data),
        }
    }
}

impl NBDSerailize for SimpleReplyMsg {
    fn to_nbd_bytes(self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1024);
        buf.put_u32(NBD_SIMPLE_REPLY_MAGIC);
        buf.put_u32(self.error as u32);
        buf.put_u64(self.handle);
        if let Some(data) = self.data {
            buf.put(data);
        }
        buf.freeze()
    }
}
