// This is about NBD_INFO

use bytes::{BufMut, Bytes, BytesMut};

use crate::{NBDInfoType, NBDSerailize, TransmissionFlags};

#[derive(Clone, Debug)]
pub enum NBDInfo {
    Export(u64, TransmissionFlags),
    Name(String),
    Description(String),
    BlockSize(u32, u32, u32),
}

impl NBDSerailize for NBDInfo {
    fn to_nbd_bytes(self) -> Bytes {
        let mut buf = BytesMut::with_capacity(20);
        match self {
            NBDInfo::Export(size, trans_flags) => {
                buf.put_u16(NBDInfoType::NBD_INFO_EXPORT as u16);
                buf.put_u64(size);
                buf.put_u16(trans_flags);
            }
            NBDInfo::Name(name) => {
                buf.put_u16(NBDInfoType::NBD_INFO_NAME as u16);
                buf.put(name.as_bytes());
            }
            NBDInfo::Description(desc) => {
                buf.put_u16(NBDInfoType::NBD_INFO_DESCRIPTION as u16);
                buf.put(desc.as_bytes());
            }
            NBDInfo::BlockSize(min_size, prefer_size, max_size) => {
                buf.put_u16(NBDInfoType::NBD_INFO_BLOCK_SIZE as u16);
                buf.put_u32(min_size);
                buf.put_u32(prefer_size);
                buf.put_u32(max_size);
            }
        };
        buf.freeze()
    }
}
