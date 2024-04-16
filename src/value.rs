/// Define All Const of NBD protocol
///
use num_enum::TryFromPrimitive;

pub const NBD_MAGIC: u64 = u64::from_be_bytes(*b"NBDMAGIC");
pub const NBD_OPT_MAGIC: u64 = u64::from_be_bytes(*b"IHAVEOPT");
pub const NBD_OPT_REP_MAGIC: u64 = 0x3e889045565a9;
pub const NBD_REQUEST_MAGIC: u32 = 0x25609513;
pub const NBD_SIMPLE_REPLY_MAGIC: u32 = 0x67446698;
pub const NBD_STRUCTURED_REPLY_MAGIC: u32 = 0x668e33ef;

pub type HandshakeFlags = u16;
// Handshake Flag is for server
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u16)]
pub enum HandshakeFlag {
    NBD_FLAG_FIXED_NEWSTYLE = 0x01,
    NBD_FLAG_NO_ZEROES = 0x02,
}

pub type ClientFlags = u32;
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u32)]
pub enum ClientFlag {
    NBD_FLAG_C_FIXED_NEWSTYLE = 0x01,
    NBD_FLAG_C_NO_ZEROES = 0x02,
}

pub type TransmissionFlags = u16;
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u16)]
pub enum TransmissionFlag {
    NBD_FLAG_HAS_FLAGS = 1 << 0, // MUST set
    NBD_FLAG_READ_ONLY = 1 << 1,
    NBD_FLAG_SEND_FLUSH = 1 << 2,
    NBD_FLAG_SEND_FUA = 1 << 3,
    NBD_FLAG_ROTATIONAL = 1 << 4,
    NBD_FLAG_SEND_TRIM = 1 << 5,
    NBD_FLAG_SEND_WRITE_ZEROES = 1 << 6,
    NBD_FLAG_SEND_DF = 1 << 7,
    NBD_FLAG_CAN_MULTI_CONN = 1 << 8,
    NBD_FLAG_SEND_RESIZE = 1 << 9,
    NBD_FLAG_SEND_CACHE = 1 << 10,
    NBD_FLAG_SEND_FAST_ZERO = 1 << 11,
}
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u32)]
pub enum OptionType {
    NBD_OPT_EXPORT_NAME = 1,
    NBD_OPT_ABORT = 2,
    NBD_OPT_LIST = 3,
    NBD_OPT_PEEK_EXPORT = 4,
    NBD_OPT_STARTTLS = 5,
    NBD_OPT_INFO = 6,
    NBD_OPT_GO = 7,
    NBD_OPT_STRUCTURED_REPLY = 8,
    NBD_OPT_LIST_META_CONTEXT = 9,
    NBD_OPT_SET_META_CONTEXT = 10,
}
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u32)]
pub enum OptionReplyType {
    NBD_REP_ACK = 1,
    NBD_REP_SERVER = 2,
    NBD_REP_INFO = 3,
    NBD_REP_META_CONTEXT = 4,
    NBD_REP_ERR_UNSUP = 2 ^ 31 + 1,
    NBD_REP_ERR_POLICY = 2 ^ 31 + 2,
    NBD_REP_ERR_INVALID = 2 ^ 31 + 3,
    NBD_REP_ERR_PLATFORM = 2 ^ 31 + 4,
    NBD_REP_ERR_TLS_REQD = 2 ^ 31 + 5,
    NBD_REP_ERR_UNKNOWN = 2 ^ 31 + 6,
    NBD_REP_ERR_SHUTDOWN = 2 ^ 31 + 7,
    NBD_REP_ERR_BLOCK_SIZE_REQD = 2 ^ 31 + 8,
    NBD_REP_ERR_TOO_BIG = 2 ^ 31 + 9,
}

#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u16)]
pub enum NBDInfoType {
    NBD_INFO_EXPORT = 0,
    NBD_INFO_NAME = 1,
    NBD_INFO_DESCRIPTION = 2,
    NBD_INFO_BLOCK_SIZE = 3,
}

pub type CommandFlags = u16;
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u16)]
pub enum CommandFlag {
    NBD_CMD_FLAG_FUA = 0x01,
    NBD_CMD_FLAG_NO_HOLE = 0x02,
    NBD_CMD_FLAG_DF = 0x04,
    NBD_CMD_FLAG_REQ_ONE = 0x08,
    NBD_CMD_FLAG_FAST_ZERO = 0x10,
}

#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u16)]
pub enum StructuredReplyFlag {
    NBD_REPLY_FLAG_DONE = 0x01,
}
#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u32)]
pub enum StucturedReplyType {
    NBD_REPLY_TYPE_NONE = 0,
    NBD_REPLY_TYPE_OFFSET_DATA = 1,
    NBD_REPLY_TYPE_OFFSET_HOLE = 2,
    NBD_REPLY_TYPE_BLOCK_STATUS = 5,
    NBD_REPLY_TYPE_ERROR = 2 ^ 15 + 1,
    NBD_REPLY_TYPE_ERROR_OFFSET = 2 ^ 15 + 2,
}

#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u16)]
pub enum RequestType {
    NBD_CMD_READ = 0,
    NBD_CMD_WRITE = 1,
    NBD_CMD_DISC = 2,
    NBD_CMD_FLUSH = 3,
    NBD_CMD_TRIM = 4,
    NBD_CMD_CACHE = 5,
    NBD_CMD_WRITE_ZEROES = 6,
    NBD_CMD_BLOCK_STATUS = 7,
    NBD_CMD_RESIZE = 8,
    NBD_CMD_OTHER_REQUESTS = 9,
}

#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u32)]
pub enum ErrorType {
    NO_ERROR = 0,
    NBD_EPERM = 1,
    NBD_EIO = 5,
    NBD_ENOMEN = 12,
    NBD_EINVAL = 22,
    NBD_ENOSPC = 28,
    NBD_EOVERFLOW = 75,
    NBD_ENOTSUP = 95,
    NBD_ESHUTDOWN = 108,
}
