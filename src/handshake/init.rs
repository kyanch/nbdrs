use anyhow::{Ok, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    handshake::info, NBDDeserailize, NBDInfoType, NBDSerailize, OptionReplyType, OptionType,
    NBD_OPT_MAGIC, NBD_OPT_REP_MAGIC,
};

use super::info::NBDInfo;

// opt magic u64;option u32;len u32;data;
#[derive(Clone, Debug)]
pub enum OptionHaggle {
    ExportName(Vec<u8>),
    Abort,
    List,
    Info(Vec<u8>, Vec<NBDInfoType>),
    Go(Vec<u8>, Vec<NBDInfoType>),
    StructuredReply,
}
impl From<OptionHaggle> for OptionType {
    fn from(value: OptionHaggle) -> Self {
        match value {
            OptionHaggle::ExportName(_) => OptionType::NBD_OPT_EXPORT_NAME,
            OptionHaggle::Abort => OptionType::NBD_OPT_ABORT,
            OptionHaggle::List => OptionType::NBD_OPT_LIST,
            OptionHaggle::Info(_, _) => OptionType::NBD_OPT_INFO,
            OptionHaggle::Go(_, _) => OptionType::NBD_OPT_GO,
            OptionHaggle::StructuredReply => OptionType::NBD_OPT_STRUCTURED_REPLY,
        }
    }
}

impl<'a, T: AsyncRead + Unpin + Send> NBDDeserailize<'a, T> for OptionHaggle {
    fn from_stream(stream: &'a mut T) -> BoxFuture<'a, Result<Self>>
    where
        Self: Sized,
    {
        let code = async {
            let magic = stream.read_u64().await?;
            if magic != NBD_OPT_MAGIC {
                return Err(anyhow!(
                    "The magic value: {:X} is NOT NBD_OPT_MAGIC!",
                    magic
                ));
            }
            let opt = stream.read_u32().await?;
            let len = stream.read_u32().await?;
            let opt = match opt.try_into()? {
                OptionType::NBD_OPT_ABORT => {
                    assert!(len == 0);
                    Self::Abort
                }
                OptionType::NBD_OPT_EXPORT_NAME => todo!(),
                OptionType::NBD_OPT_LIST => {
                    assert!(len == 0);
                    Self::List
                }
                OptionType::NBD_OPT_PEEK_EXPORT => todo!(),
                OptionType::NBD_OPT_STARTTLS => todo!(),
                OptionType::NBD_OPT_INFO => {
                    let mut buf = BytesMut::zeroed(len as usize);
                    stream.read_exact(&mut buf).await?;
                    // read info data
                    let name_len = buf.get_u32();
                    let name = buf[..name_len as usize].to_vec();

                    let info_count = buf.get_u16();
                    let mut infos = Vec::new();
                    for _ in 0..info_count {
                        infos.push(buf.get_u16().try_into()?);
                    }
                    // finish info data
                    OptionHaggle::Info(name, infos)
                }
                OptionType::NBD_OPT_GO => {
                    // same with NBD_OPT_INFO
                    let mut buf = BytesMut::zeroed(len as usize);
                    stream.read_exact(&mut buf).await?;
                    // read info data
                    let name_len = buf.get_u32();
                    let name = buf[..name_len as usize].to_vec();

                    let info_count = buf.get_u16();
                    let mut infos = Vec::new();
                    for _ in 0..info_count {
                        infos.push(buf.get_u16().try_into()?);
                    }
                    // finish info data
                    OptionHaggle::Go(name, infos)
                }
                OptionType::NBD_OPT_STRUCTURED_REPLY => todo!(),
                OptionType::NBD_OPT_LIST_META_CONTEXT => todo!(),
                OptionType::NBD_OPT_SET_META_CONTEXT => todo!(),
            };
            Ok(opt)
        };
        Box::pin(code)
    }
}

impl NBDSerailize for OptionHaggle {
    fn to_nbd_bytes(self) -> Bytes {
        let mut buf = BytesMut::with_capacity(20);
        buf.put_u64(NBD_OPT_MAGIC);
        match self {
            OptionHaggle::ExportName(name) => {
                buf.put_u32(OptionType::NBD_OPT_EXPORT_NAME as u32);
                buf.put_u32(name.len() as u32);
                buf.put(&name[..]);
            }
            OptionHaggle::Abort => {
                buf.put_u32(OptionType::NBD_OPT_ABORT as u32);
                buf.put_u32(0);
            }
            OptionHaggle::List => todo!(),
            OptionHaggle::Info(name, infos) => {
                buf.put_u32(OptionType::NBD_OPT_INFO as u32);
                buf.put_u32((4 + name.len() + 2 + 2 * infos.len()) as u32);
                // build info data
                buf.put_u32(name.len() as u32);
                buf.put(&name[..]);
                buf.put_u16(infos.len() as u16);
                infos.into_iter().for_each(|i| buf.put_u32(i as u32));
                // build finish
            }
            OptionHaggle::Go(name, infos) => {
                buf.put_u32(OptionType::NBD_OPT_GO as u32);
                buf.put_u32((4 + name.len() + 2 + 2 * infos.len()) as u32);
                // build info data
                buf.put_u32(name.len() as u32);
                buf.put(&name[..]);
                buf.put_u16(infos.len() as u16);
                infos.into_iter().for_each(|i| buf.put_u32(i as u32));
                // build finish
            }
            OptionHaggle::StructuredReply => {
                buf.put_u32(OptionType::NBD_OPT_STRUCTURED_REPLY as u32);
                buf.put_u32(0);
            }
        }
        buf.freeze()
    }
}

#[derive(Clone, Debug)]
pub struct OptionHaggleRep {
    client_option: OptionType,
    reply: _InnerOptionHaggleRepType,
}
#[derive(Clone, Debug)]
enum _InnerOptionHaggleRepType {
    None,
    ACK,
    Server(String),
    Info(NBDInfo),
    MetaContext(u32, String),
    Unsupport(Option<String>),
    PolicyErr(Option<String>),
    Invalid(Option<String>),
    PlatformErr(Option<String>),
}
impl OptionHaggleRep {
    pub fn with_option(client_option: OptionType) -> Self {
        OptionHaggleRep {
            client_option,
            reply: _InnerOptionHaggleRepType::None,
        }
    }
    pub fn ack(&mut self) {
        self.reply = _InnerOptionHaggleRepType::ACK;
    }
    pub fn server(&mut self, name: String) {
        self.reply = _InnerOptionHaggleRepType::Server(name);
    }
    pub fn info(&mut self, info: NBDInfo) {
        self.reply = _InnerOptionHaggleRepType::Info(info);
    }
    pub fn unsupport(&mut self) {
        self.reply = _InnerOptionHaggleRepType::Unsupport(None);
    }
    pub fn unsupport_with_msg(&mut self, err: String) {
        self.reply = _InnerOptionHaggleRepType::Unsupport(Some(err));
    }
}

impl NBDSerailize for OptionHaggleRep {
    fn to_nbd_bytes(self) -> Bytes {
        let mut buf = BytesMut::with_capacity(20);
        buf.put_u64(NBD_OPT_REP_MAGIC);
        buf.put_u32(self.client_option as u32);
        match self.reply {
            _InnerOptionHaggleRepType::ACK => {
                buf.put_u32(OptionReplyType::NBD_REP_ACK as u32);
                buf.put_u32(0);
            }
            _InnerOptionHaggleRepType::Server(server_name) => {
                buf.put_u32(OptionReplyType::NBD_REP_SERVER as u32);
                buf.put_u32(server_name.as_bytes().len() as u32);
                buf.put(server_name.as_bytes());
            }
            _InnerOptionHaggleRepType::Info(info) => {
                buf.put_u32(OptionReplyType::NBD_REP_INFO as u32);
                let info = info.to_nbd_bytes();
                buf.put_u32(info.len() as u32);
                buf.put(info);
            }
            _InnerOptionHaggleRepType::MetaContext(_, _) => todo!(),
            _InnerOptionHaggleRepType::Unsupport(msg) => {
                buf.put_u32(OptionReplyType::NBD_REP_ERR_UNSUP as u32);
                if let Some(msg) = msg {
                    let msg = msg.as_bytes();
                    buf.put_u32(msg.len() as u32);
                    buf.put(msg);
                } else {
                    buf.put_u32(0);
                }
            }
            _InnerOptionHaggleRepType::PolicyErr(_) => todo!(),
            _InnerOptionHaggleRepType::Invalid(_) => todo!(),
            _InnerOptionHaggleRepType::PlatformErr(_) => todo!(),
            _InnerOptionHaggleRepType::None => {
                eprintln!("OptionHaggleRep type is NONE!Use UNSUP instead!");
                let info = "internal error";
                buf.put_u32(OptionReplyType::NBD_REP_ERR_UNSUP as u32);
                buf.put_u32(info.as_bytes().len() as u32);
                buf.put(info.as_bytes());
            }
        }
        buf.freeze()
    }
}
