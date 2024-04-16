
use anyhow::{Ok, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufStream};

use crate::{
    handshake::{
        info::NBDInfo,
        init::{OptionHaggle, OptionHaggleRep},
    },
    transmission::{RequestMsg, SimpleReplyMsg},
    ErrorType, HandshakeFlag, HandshakeFlags, NBDDeserailize, NBDSerailize, RequestType,
    TransmissionFlag, TransmissionFlags, NBD_MAGIC, NBD_OPT_MAGIC,
};

pub struct Server<T: AsyncRead + AsyncWrite + Unpin + Send> {
    stream: BufStream<T>,
    size: u64,
    server_flags: HandshakeFlags,
    transmission_flags: TransmissionFlags,
    data: Vec<u8>,
}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Server<T> {
    pub fn new(stream: T) -> Self {
        let size = 1024 * 1024 * 256;
        let mut data = Vec::new();
        data.resize(size, 0);
        Server {
            stream: BufStream::new(stream),
            size: size as u64,
            server_flags: HandshakeFlag::NBD_FLAG_FIXED_NEWSTYLE as HandshakeFlags,
            transmission_flags: TransmissionFlag::NBD_FLAG_HAS_FLAGS as TransmissionFlags,
            data: data,
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin + Send> Server<T> {
    pub async fn handshake(&mut self) -> Result<()> {
        // first stage
        self.stream.write_u64(NBD_MAGIC).await?;
        self.stream.write_u64(NBD_OPT_MAGIC).await?;
        self.stream.write_u16(self.server_flags).await?;
        self.stream.flush().await?;
        let client_flags = self.stream.read_u32().await?;
        // TODO: client_flags check
        println!("client flags: {:x}", client_flags);

        // second stage: hanggle options
        self.handle_option().await?;

        Ok(())
    }
    pub async fn handle_option(&mut self) -> Result<()> {
        let opt_haggle = OptionHaggle::from_stream(&mut self.stream).await?;

        // 只有option，没有具体的回复内容
        let mut reply_empty = OptionHaggleRep::with_option(opt_haggle.clone().into());

        match opt_haggle {
            OptionHaggle::Abort => {
                reply_empty.ack();
                self.stream.write(&reply_empty.to_nbd_bytes()).await?;
                self.stream.flush().await?;
            }
            OptionHaggle::List => {
                reply_empty.ack();
                self.stream.write(&reply_empty.to_nbd_bytes()).await?;
                self.stream.flush().await?;
            }
            OptionHaggle::Info(_name, _infos) => {
                let mut reply = reply_empty.clone();
                reply.info(NBDInfo::Export(self.size, self.transmission_flags));
                self.stream.write(&reply.to_nbd_bytes()).await?;

                let mut reply = reply_empty.clone();
                reply.ack();
                self.stream.write(&reply.to_nbd_bytes()).await?;

                self.stream.flush().await?;
            }
            OptionHaggle::Go(_name, _infos) => {
                let mut reply = reply_empty.clone();
                reply.info(NBDInfo::Export(self.size, self.transmission_flags));
                self.stream.write_all(&reply.to_nbd_bytes()).await?;

                let mut reply = reply_empty.clone();
                reply.ack();
                self.stream.write_all(&reply.to_nbd_bytes()).await?;

                self.stream.flush().await?;
                return Ok(());
            }
            _ => {
                reply_empty.unsupport();
                self.stream.write_all(&reply_empty.to_nbd_bytes()).await?;
            }
        };
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn handle_transmission(&mut self) -> Result<()> {
        loop {
            let mut requset = RequestMsg::from_stream(&mut self.stream).await?;
            let reply = match &requset.r#type {
                RequestType::NBD_CMD_READ => {
                    let start = requset.offset as usize;
                    let end = start + requset.length as usize;
                    if end as u64 > self.size {
                        SimpleReplyMsg::with_err(&requset, ErrorType::NBD_ENOSPC)
                    } else {
                        let data = self.data[start..end].to_vec().into();
                        SimpleReplyMsg::with_data(&requset, data)
                    }
                }
                RequestType::NBD_CMD_WRITE => {
                    let start = requset.offset as usize;
                    let end = start + requset.length as usize;
                    let data = requset.data.take().unwrap();
                    assert!(requset.length as usize == data.len());
                    self.data[start..end].copy_from_slice(&data);
                    SimpleReplyMsg::new(&requset)
                }
                RequestType::NBD_CMD_DISC => return Ok(()),
                _ => SimpleReplyMsg::with_err(&requset, ErrorType::NBD_ENOTSUP),
            };
            self.stream.write_all(&reply.to_nbd_bytes()).await?;
            self.stream.flush().await?;
        }
    }
}
