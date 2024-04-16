# NBD 协议

> 此文档主要为 [proto.md](https://github.com/NetworkBlockDevice/nbd/blob/master/doc/proto.md) 的机翻，部分涉及协议实现的部分会自己渣翻，仅供参考。英文原文档也拷贝到了本仓库

## 介绍

网络块设备是一个源于 Linux 的轻量块设备访问协议，它运行将块设备导出到客户端。虽然协议名字里特别提到了块设备，但该*协议*没有任何要求导出的设备实际上是块设备。协议只涉及一个特定的字节范围和该范围内的特定偏移位置上执行几个特定长度的操作。

为了清晰起见，在本文档中，我们称服务器上的导出称为块设备，尽管服务器上的实际后端可能不是实际的块设备；它可能是一个块设备，一个普通文件，或者涉及多个文件的更复杂配置。这是服务器的一个实现细节。

## 约定

在下面的协议描述中，标签 "C: "用于表示客户端发送的信息，而 "S: "用于表示客户端发送的信息。
`等宽文本`用于表示字面字符数据或（在注释中使用时）常量名，`0xdeadbeef`用于表示字面上的十六进制数（这些数字总是以网络字节顺序发送），而（括号）用于添加注释。文档中除此之外的内容都是对发送的数据的描述。

本文档中的关键词 "必须","不能", "REQUIRED", "SHALL",
"SHALL NOT", "应该", "RECOMMENDED",
"可以", and "OPTIONAL" 应按照 [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt) 中的描述来理解。

当本文档提到一个字符串时，除非另有说明，该字符串是一个 UTF-8 码点序列，不以`NUL`字符结束，不得包含`NUL`字符， **应该** 不超过 256 字节长，且 **必须** 不超过 4096 字节长。这适用于导出名称和错误信息等。字符串的长度总是可以事先取得，尽管可能需要根据同一消息中出现的其他数据的大小进行一些计算。

## 协议阶段

NBD 协议分为两个阶段：握手和传输。在握手期间，客户端和服务器之间会建立连接，并协商导出 NBD 设备和其他协议参数。握手成功后，客户机和服务器进入传输阶段，在这一阶段中，导出设备被读写。

在 Linux 下的客户端，握手是在用户空间实现的，而传输阶段是在内核空间实现的。要从握手阶段进入传输阶段，客户端需要执行：
ioctl(nbd, NBD_SET_SOCK, sock)
ioctl(nbd, NBD_DO_IT)

上述命令中中的 `nbd` 是打开的 `/dev/nbdX` 设备节点的文件描述符，而 `sock` 是连接服务器的套接字。在客户端断开连接之前，上述两个调用中的第二个调用不会返回。

请注意，客户端还可以使用其他 `ioctl` 调用，将在握手过程中与服务器协商好的选项传递给内核。在握手过程中与服务器协商的选项。本文档不描述这些选项。

使用 Linux 内核处理客户端传输阶段时，客户端和服务器之间的套接字可以使用 Unix 或 TCP 套接字。对于其他实现方式，客户端和服务器可以使用任何双方同意的通信通道（套接字是典型的方式，但也可以通过一对单向管道实现 NBD 协议）。如果使用 TCP 套接字，客户机和服务器都应禁用 Nagle 算法（即使用 `setsockopt` 将 `TCP_NODELAY` 选项设置为非零），以消除大消息有效载荷跨越多个网络数据包时等待 ACK 响应所造成的人为延迟。

### 握手阶段

握手是协议的第一阶段。它的主要目的是为客户端和服务器提供协商使用哪个出口以及如何使用的手段。

协议有三个版本。它们被称为 "oldstyle"、"newstyle "和 "fixed newstyle" 协议。在 nbd 2.9.16 之前，oldstyle 是唯一的协商版本；nbd 2.9.17 引入了 newstyle。不久后，人们发现 newstyle 的结构不足以在保留向后兼容性的同时添加协议选项。为解决这一问题而引入的微小改动在必要时被称为 "fixed newstyle"，以区别于原始版本的 newstyle 协商。

#### Oldstyle negotiation

S: 64 bits, `0x4e42444d41474943` (ASCII '`NBDMAGIC`') (also known as
the `INIT_PASSWD`)  
S: 64 bits, `0x00420281861253` (`cliserv_magic`, a magic number)  
S: 64 bits, size of the export in bytes (unsigned)  
S: 32 bits, flags  
S: 124 bytes, zeroes (reserved).

As can be seen, this isn't exactly a negotiation; it's just a matter of
the server sending a bunch of data to the client. If the client is
unhappy with what he receives, he should disconnect and not look back.

The fact that the size of the export was specified before the flags were
sent, made it impossible for the protocol to be changed in a
backwards-compatible manner to allow for named exports without ugliness.
As a result, the old style negotiation is now no longer developed;
从参考实现的 3.10 版开始，不再支持 Oldstyle negotiation。

#### Newstyle negotiation

希望使用新协商方式的客户端应通过 IANA 为 NBD 保留的端口 10809 进行连接。服务器也可以监听其他端口，但应在这些端口上使用 oldstyle 握手方式。服务器应拒绝在新式端口上进行旧式协商。出于调试目的，服务器可以更改监听新式协商的端口，但这不应用于生产目的。

Newstyle 协议最初的几个消息如下:

S: 64 bits, `0x4e42444d41474943` (ASCII '`NBDMAGIC`') (和 oldstyle 相同)  
S: 64 bits, `0x49484156454F5054` (ASCII '`IHAVEOPT`') (注意：和 oldstyle 不同)  
S: 16 bits, handshake flags  
C: 32 bits, client flags

至此，协商的初始阶段结束；客户机和服务器现在都知道自己理解的是新式握手的第一个版本，没有任何可选项。客户端 **应该** 忽略任何它无法识别的握手标记(flags)，而服务端如果无法识别客户端的标记(flags)，则 **必须** 关闭 TCP 连接。

下面是一组重复的 options。在`非fixed newstyle`中，只能设置一个选项（`NBD_OPT_EXPORT_NAME`），而且是必须的。
在此阶段，我们进入选项协商过程，期间客户端可以向服务器发送一个或者（在`fixed newstyle`中允许）多个选项(option)。设置选项的一般格式如下：

C: 64 bits, `0x49484156454F5054` (ASCII '`IHAVEOPT`') (和 newstyle 握手协议一样)  
C: 32 bits, option
C: 32 bits, length of option data (unsigned)
C: any data needed for the chosen option, of length as specified above.

每个 option 中的 选项长度 允许服务器跳过任何它不理解的选项。

如果选项字段的值为 "NBD_OPT_EXPORT_NAME"，且服务器允许导出，则服务器将回复导出的相关信息：

S: 64 bits, size of the export in bytes (unsigned)  
S: 16 bits, transmission flags  
S: 124 bytes, zeroes (reserved) (unless `NBD_FLAG_C_NO_ZEROES` was
negotiated by the client)

如果服务器不愿意允许导出，则**必须**终止会话。

标记(flags)字段之所以是 16 位，而不是 oldstyle 中的 32 位，是因为现在有 16 位传输标记和 16 位握手标记。两者相加为 32 位，这样就可以为两者使用一套通用的宏。如果我们用完了标志位，服务器将设置标志位的最高位，表示随后会有一个额外的标志字段，客户端必须在额外的标志发送之前回复一个自己的标志字段。这一机制尚未实施。

#### Fixed newstyle negotiation

不幸的是，由于一个错误，当服务器遇到一个不理解的选项时，它会立即关闭连接，而不是向客户端发出一个消息，客户端本可以利用这个消息来重试连接；而且服务器的回复也没有结构化，这意味着如果服务器发送了客户端不理解的内容，客户端也必须终止协议。

为了解决这两个问题，实施了以下改变：

- 服务器将设置握手标志`NBD_FLAG_FIXED_NEWSTYLE`，以表示它支持`fixed newstyle`协商。
- 客户端应在其标志字段中也设置 `NBD_FLAG_C_FIXED_NEWSTYLE` 来回复，但其协议内容仍然兼容 `newstyle`。
- 根据上述描述的发送选项的通用格式，客户端现在**可以**适当地向服务器发送其他选项。
- 对于除 `NBD_OPT_EXPORT_NAME` 以外的任何选项，服务器都将以下列格式回复数据包：

S: 64 bits, `0x3e889045565a9` (magic number for replies)  
S: 32 bits, the option as sent by the client to which this is a reply  
S: 32 bits, reply type (e.g., `NBD_REP_ACK` for successful completion,
or `NBD_REP_ERR_UNSUP` to mark use of an option not known by this
server  
S: 32 bits, length of the reply. This MAY be zero for some replies, in
which case the next field is not sent  
S: any data as required by the reply (e.g., an export name in the case
of `NBD_REP_SERVER`)

客户端在收到对其发送的任何选项的最终回复之前，**必须**不发送任何选项（注意，某些选项（如 `NBD_OPT_LIST`）有多个回复，最终回复是其中的最后一个）。

客户端发送的某些信息会指示服务器更改其某些内部状态。 客户端不应多次发送此类消息；如果客户端发送了此类消息，服务器可能会以 `NBD_REP_ERR_INVALID` 来使重复发送的消息失败。

#### 结束选项交换 Termination of the session during option haggling

有三个机制来结束选项交换：

- 可以进入传输模式（通过客户端发送 `NBD_OPT_EXPORT_NAME`，或通过服务器以 `NBD_REP_ACK`响应 `NBD_OPT_GO`）。这一点在其他地方有详细说明。

- 客户端可以发送（服务器也可以回复）"NBD_OPT_ABORT"。随后，客户端**必须**关闭 TLS（如果正在运行），并放弃连接。这被称为 "启动软断开"；软断开只能由客户端发起。

- 客户端或服务器可以断开 TCP 会话，而无需在 NBD 协议层进行任何操作。如果协商了 TLS，发起交易的一方**应该**首先关闭正在运行的 TLS。这被称为 "硬断开连接"。

本节涉及其中的第二项和第三项，合称 "终止会话"，以及在哪些情况下有效。

如果客户机或服务器检测到对方违反了强制性条件（"**必须**"等），则**可以**启动硬断开连接。

客户端**可以**随时使用软断开连接来终止会话。

本文件规定必须终止会话的一方，在无法使用软断开连接的情况下，**必须**启动硬断开连接。这些情况包括：该方是服务器，且无法返回错误（例如，在无法满足 `NBD_OPT_EXPORT_NAME` 之后），以及该方是 TLS 协商失败后的客户端。

除本节规定的情况外，一方**不能**主动硬断开连接。因此，除非客户的情况符合上一段的规定，或客户检测到违反了强制性条件，否则**不能**使用硬断开，因此客户终止会话的唯一选择是软断开。

如果客户端或服务器不希望完成协商，则无需这样做。任何一方都**可以**直接终止会话。在客户端的情况下，如果它希望这样做，就**必须**使用软断开连接。

在服务器的情况下，它**必须**（除上述情况外）简单地对入站（inbound）选项进行错误处理，直到客户端意识到它不受欢迎为止。
如果服务器认为客户端的行为构成拒绝服务(DOS)，则**可以**启动硬断开连接。
如果服务器正在关闭过程中，它**可以**对任何入站选项进行错误处理，并应使用`NBD_REP_ERR_SHUTDOWN`对收到的其他选项（`NBD_OPT_ABORT`除外）进行错误处理。

如果客户端收到 "NBD_REP_ERR_SHUTDOWN"，则**必须**启动软断开。

### 传输阶段

传输阶段中有三种消息类型：请求、简单回复和结构化回复块。传输阶段包括一系列交易，客户端提交请求，服务器针对每个请求发送相应的回复，回复可以是单个简单回复或一个或多个结构化回复块的系列。该阶段持续到任一方终止传输；这只能由客户端干净地执行。

请注意，如果没有客户端协商，服务器**必须**只使用简单回复，而且仅凭阅读服务器流量是无法判断是否会有数据字段存在的；简单回复也无法有效处理`NBD_CMD_READ`请求的错误。因此，结构化回复可以用来创建一个无上下文的服务器流；见下文。

回复不需要按请求的顺序发送（即，请求可能被服务器异步处理），并且一个请求的结构化回复块可能与其他请求的回复消息交错；然而，可能有限制阻止在给定回复中任意重排序结构化回复块。客户端**应该**使用与所有其他当前挂起的事务都不同的句柄，但**可以**重用不再使用的句柄；句柄不需要连续。在每个回复消息中（无论是简单的还是结构化的），服务器**必须**使用客户端在相应请求中发送的相同句柄值。通过这种方式，客户端可以关联哪个请求收到了响应。

#### 消息读写顺序

服务器**可以**无序处理命令，并且**可以**无序回复，但有以下例外：

- 所有写命令（包括`NBD_CMD_WRITE`、`NBD_CMD_WRITE_ZEROES`和`NBD_CMD_TRIM`），服务器在处理一个`NBD_CMD_FLUSH`之前完成（即回复）的，必须在回复那个`NBD_CMD_FLUSH`之前，被写入非易失性存储。这一段只在`NBD_FLAG_SEND_FLUSH`被设置在传输标志中时适用，否则客户端永远不会向服务器发送`NBD_CMD_FLUSH`。

- 使用多个连接到服务器以并行化命令的客户端，在收到其期望被 flush 覆盖的所有写命令的回复之前，**不得**发出`NBD_CMD_FLUSH`请求。

- 服务器在命令标志中设置了`NBD_CMD_FLAG_FUA`的命令的数据（如果有的话）被持久化到非易失性存储之前，**不得**回复该命令。这只在`NBD_FLAG_SEND_FUA`被设置在传输标志中时适用，否则`NBD_CMD_FLAG_FUA`不会被设置在任何客户端发送给服务器的命令上。

`NBD_CMD_FLUSH`是基于 Linux 内核中设置了`REQ_PREFLUSH`的空 bio 建模的。
NBD_CMD_FLAG_FUA 是基于 Linux 内核中设置了`REQ_FUA`的 bio 建模的。如果这个规范中有歧义，[kernel documentation](https://www.kernel.org/doc/Documentation/block/writeback_cache_control.txt)可能有用。

#### Request message

The request message, sent by the client, looks as follows:

C: 32 bits, 0x25609513, magic (`NBD_REQUEST_MAGIC`)  
C: 16 bits, command flags  
C: 16 bits, type  
C: 64 bits, handle  
C: 64 bits, offset (unsigned)  
C: 32 bits, length (unsigned)  
C: (*length* bytes of data if the request is of type `NBD_CMD_WRITE`)

#### 简单回复消息

如果没有通过`NBD_OPT_STRUCTURED_REPLY`协商结构化回复，服务器**必须**对所有请求发送简单回复消息。如果已经协商了结构化回复，除了`NBD_CMD_READ`请求之外，**可以**对任何没有数据负载的请求使用简单回复。消息格式如下：

S: 32 bits, 0x67446698, magic (`NBD_SIMPLE_REPLY_MAGIC`; used to be
`NBD_REPLY_MAGIC`)  
S: 32 bits, error (MAY be zero)  
S: 64 bits, handle  
S: (*length* bytes of data if the request is of type `NBD_CMD_READ` and
*error* is zero)

#### 结构化回复块消息

对`NBD_CMD_READ`的默认简单回复的主要缺点如下。首先，它不支持部分读取或早期错误（命令必须整体成功或失败，要么发送长度字节的数据，要么即使是因为错误标志导致的`NBD_EINVAL`错误也必须启动硬断开）。其次，没有有效的方法跳过已知全为零的稀疏文件部分。最后，如果没有客户端发送的待处理读请求的上下文，就无法可靠地解码服务器流量。因此，如果经过协商，也允许使用结构化回复。

在传输阶段，一个结构化回复包括一个或多个结构化回复块消息。除非客户端通过`NBD_OPT_STRUCTURED_REPLY`成功协商结构化回复，否则服务器**不得**发送此类回复类型。相反，如果协商了结构化回复，服务器**必须**对任何带有有效载荷的响应使用结构化回复，并且**不得**对`NBD_CMD_READ`使用简单回复（即使是因为错误标志导致的早期`NBD_EINVAL`错误），但**可以**对所有其他请求使用简单回复或结构化回复。服务器**应该**倾向于通过结构化回复发送错误，因为错误可以随后通过字符串有效载荷呈现给用户。

结构化回复**可以**占用多个结构化块消息（所有这些消息的“handle”值相同），并且使用`NBD_REPLY_FLAG_DONE`回复标志来标识最终块。
除非个别请求有在下面有进一步的文档记录，否则块**可以**以任何顺序发送，但是带有`NBD_REPLY_FLAG_DONE`标志的块**必须**最后发送。即使命令在一个回复的块之间记录了更多的约束，与其他请求相关的消息交错插入该回复的块始终是安全的。
服务器**应该**尽量减少一个回复中块的数量，但如果在该块的传输完成前还有检测到错误的可能，就**不得**将块标记为最终块。
只有在不包含任何错误块的情况下，结构化回复才被视为成功，尽管客户端**可能**根据接收到的块判断部分成功。

结构化回复块消息的格式如下：、

S: 32 bits, 0x668e33ef, magic (`NBD_STRUCTURED_REPLY_MAGIC`)  
S: 16 bits, flags  
S: 16 bits, type  
S: 64 bits, handle  
S: 32 bits, length of payload (unsigned)  
S: *length* bytes of payload data (if *length* is nonzero)

在回复中使用*length*字段可以在没有上下文的情况下，将服务器的整体流量划分为单独的回复消息；*type*字段描述了如何进一步解释有效载荷。

#### 传输阶段的终止

有两种方式来终止传输阶段：

- 客户端发送`NBD_CMD_DISC`命令后，服务器**必须**关闭TLS会话（如果正在运行的话），然后关闭TCP连接。这称为“发起软断开”。软断开只能由客户端发起。

- 当客户端或服务器终止TCP会话时（在这种情况下，它**应该**首先关闭TLS会话），这被称为“发起硬断开”。

这些行为被统称为“终止传输”。

如果一方检测到另一方违反了本文档中的强制性条件，任何一方都**可以**发起硬断开。

在服务器关闭时，服务器**应该**等待正在处理中的请求得到处理后再发起硬断开。服务器**可以**通过发出错误回复来加速这一过程。对于这些请求以及任何后续收到的请求，**应该**发出的错误值应该是`NBD_ESHUTDOWN`。

如果客户端收到一个`NBD_ESHUTDOWN`错误，它**必须**发起一个软断开。

客户端**可以**随时发起软断开，但**应该**等到没有正在处理中的请求时再进行。

客户端和服务器**不得**在上述情况之外的任何情况下发起任何形式的断开连接。

#### Reserved Magic values

The following magic values are reserved and must not be used
for future protocol extentions:

0x12560953 - Historic value for NBD_REQUEST_MAGIC, used
until Linux 2.1.116pre2.
0x96744668 - Historic value for NBD_REPLY_MAGIC, used
until Linux 2.1.116pre2.
0x25609514 - Used by nbd-server to store data log flags in the
transaction log. Never sent from/to a client.

## TLS support

The NBD protocol supports Transport Layer Security (TLS) (see
[RFC5246](https://tools.ietf.org/html/rfc5246)
as updated by
[RFC6176](https://tools.ietf.org/html/rfc6176)
).

TLS is negotiated with the `NBD_OPT_STARTTLS`
option. This is performed as an in-session upgrade. Below the term
'negotiation' is used to refer to the sending and receiving of
NBD options and option replies, and the term 'initiation' of TLS
is used to refer to the actual upgrade to TLS.

### Certificates, authentication and authorisation

This standard does not specify what encryption, certification
and signature algorithms are used. This standard does not
specify authentication and authorisation (for instance
whether client and/or server certificates are required and
what they should contain); this is implementation dependent.

TLS requires fixed newstyle negotiation to have completed.

### Server-side requirements

There are three modes of operation for a server. The
server MUST support one of these modes.

- The server operates entirely without TLS ('NOTLS'); OR

- The server insists upon TLS, and forces the client to
  upgrade by erroring any NBD options other than `NBD_OPT_STARTTLS`
  or `NBD_OPT_ABORT` with `NBD_REP_ERR_TLS_REQD` ('FORCEDTLS'); this
  in practice means that all option negotiation (apart from the
  `NBD_OPT_STARTTLS` itself) is carried out with TLS; OR

- The server provides TLS, and it is mandatory on zero or more
  exports, and is available at the client's option on all
  other exports ('SELECTIVETLS'). The server does not force
  the client to upgrade to TLS during option haggling (as
  if the client ultimately were to choose a non-TLS-only export,
  stopping TLS is not possible). Instead it permits the client
  to upgrade as and when it chooses, but unless an upgrade to
  TLS has already taken place, the server errors attempts
  to enter transmission mode on TLS-only exports, MAY
  refuse to provide information about TLS-only exports
  via `NBD_OPT_INFO`, MAY refuse to provide information
  about non-existent exports via `NBD_OPT_INFO`, and MAY omit
  exports that are TLS-only from `NBD_OPT_LIST`.

The server MAY determine the mode in which it operates
dependent upon the session (for instance it might be
more liberal with TCP connections made over the loopback
interface) but it MUST be consistent in its mode
of operation across the lifespan of a single TCP connection
to the server. A client MUST NOT assume indications from
a prior TCP session to a given server will be relevant
to a subsequent session.

The server MUST operate in NOTLS mode unless the server
set flag `NBD_FLAG_FIXED_NEWSTYLE` and the client replied
with `NBD_FLAG_C_FIXED_NEWSTYLE` in the fixed newstyle
negotiation.

These modes of operations are described in detail below.

#### NOTLS模式

如果服务器接收到`NBD_OPT_STARTTLS`，它必须回复`NBD_REP_ERR_POLICY`（如果由于政策原因不支持TLS）、`NBD_REP_ERR_UNSUP`（如果根本不支持`NBD_OPT_STARTTLS`选项）或本文档明确允许的其他错误。服务器不得对任何选项请求回复`NBD_REP_ERR_TLS_REQD`。

#### FORCEDTLS mode

If the server receives `NBD_OPT_STARTTLS` prior to negotiating
TLS, it MUST reply with `NBD_REP_ACK`. If the server receives
`NBD_OPT_STARTTLS` when TLS has already been negotiated, it
it MUST reply with `NBD_REP_ERR_INVALID`.

After an `NBD_REP_ACK` reply has been sent, the server MUST be
prepared for a TLS handshake, and all further data MUST be sent
and received over TLS. There is no downgrade to a non-TLS session.

As per the TLS standard, the handshake MAY be initiated either
by the server (having sent the `NBD_REP_ACK`) or by the client.
If the handshake is unsuccessful (for instance the client's
certificate does not match) the server MUST terminate the
session as by this stage it is too late to continue without TLS
as the acknowledgement has been sent.

If the server receives any other option, including `NBD_OPT_INFO`
and unsupported options, it MUST reply with `NBD_REP_ERR_TLS_REQD`
if TLS has not been initiated; `NBD_OPT_INFO` is included as in this
mode, all exports are TLS-only. If the server receives a request to
enter transmission mode via `NBD_OPT_EXPORT_NAME` when TLS has not
been initiated, then as this request cannot error, it MUST
terminate the session. If the server receives a request to
enter transmission mode via `NBD_OPT_GO` when TLS has not been
initiated, it MUST error with `NBD_REP_ERR_TLS_REQD`.

The server MUST NOT send `NBD_REP_ERR_TLS_REQD` in reply to
any option if TLS has already been initiated.

The FORCEDTLS mode of operation has an implementation problem in
that the client MAY legally simply send a `NBD_OPT_EXPORT_NAME`
to enter transmission mode without previously sending any options.
This is avoided by use of `NBD_OPT_INFO` and `NBD_OPT_GO`.

#### SELECTIVETLS mode

If the server receives `NBD_OPT_STARTTLS` prior to negotiating
TLS, it MUST reply with `NBD_REP_ACK` and initiate TLS as set
out under 'FORCEDTLS' above. If the server receives
`NBD_OPT_STARTTLS` when TLS has already been negotiated, it
it MUST reply with `NBD_REP_ERR_INVALID`.

If the server receives `NBD_OPT_INFO` or `NBD_OPT_GO` and TLS
has not been initiated, it MAY reply with `NBD_REP_ERR_TLS_REQD`
if that export is non-existent, and MUST reply with
`NBD_REP_ERR_TLS_REQD` if that export is TLS-only.

If the server receives a request to enter transmission mode
via `NBD_OPT_EXPORT_NAME` on a TLS-only export when TLS has not
been initiated, then as this request cannot error, it MUST
terminate the session.

The server MUST NOT send `NBD_REP_ERR_TLS_REQD` in reply to
any option if TLS has already been negotiated. The server
MUST NOT send `NBD_REP_ERR_TLS_REQD` in response to any
option other than `NBD_OPT_INFO`, `NBD_OPT_GO` and
`NBD_OPT_EXPORT_NAME`, and only in those cases in respect of
a TLS-only or non-existent export.

There is a degenerate case of SELECTIVETLS where all
exports are TLS-only. This is permitted in part to make programming
of servers easier. Operation is a little different from FORCEDTLS,
as the client is not forced to upgrade to TLS prior to any options
being processed, and the server MAY choose to give information on
non-existent exports via `NBD_OPT_INFO` responses prior to an upgrade
to TLS.

### Client-side requirements

If the client supports TLS at all, it MUST be prepared
to deal with servers operating in any of the above modes.
Notwithstanding, a client MAY always terminate the session or
refuse to connect to a particular export if TLS is
not available and the user requires TLS.

The client MUST NOT issue `NBD_OPT_STARTTLS` unless the server
set flag `NBD_FLAG_FIXED_NEWSTYLE` and the client replied
with `NBD_FLAG_C_FIXED_NEWSTYLE` in the fixed newstyle
negotiation.

The client MUST NOT issue `NBD_OPT_STARTTLS` if TLS has already
been initiated.

Subject to the above two limitations, the client MAY send
`NBD_OPT_STARTTLS` at any time to initiate a TLS session. If the
client receives `NBD_REP_ACK` in response, it MUST immediately
upgrade the session to TLS. If it receives `NBD_REP_ERR_UNSUP`,
`NBD_REP_ERR_POLICY` or any other error in response, it indicates
that the server cannot or will not upgrade the session to TLS,
and therefore the client MUST either continue the session
without TLS, or terminate the session.

A client that prefers to use TLS irrespective of whether
the server makes TLS mandatory SHOULD send `NBD_OPT_STARTTLS`
as the first option. This will ensure option haggling is subject
to TLS, and will thus prevent the possibility of options being
compromised by a Man-in-the-Middle attack. Note that the
`NBD_OPT_STARTTLS` itself may be compromised - see 'downgrade
attacks' for more details. For this reason, a client which only
wishes to use TLS SHOULD terminate the session if the
`NBD_OPT_STARTTLS` replies with an error.

If the TLS handshake is unsuccessful (for instance the server's
certificate does not validate) the client MUST terminate the
session as by this stage it is too late to continue without TLS.

If the client receives an `NBD_REP_ERR_TLS_REQD` in response
to any option, it implies that this option cannot be executed
unless a TLS upgrade is performed. If the option is any
option other than `NBD_OPT_INFO` or `NBD_OPT_GO`, this
indicates that no option will succeed unless a TLS upgrade
is performed; the client MAY therefore choose to issue
an `NBD_OPT_STARTTLS`, or MAY terminate the session (if
for instance it does not support TLS or does not have
appropriate credentials for this server). If the client
receives `NBD_REP_ERR_TLS_REQD` in response to
`NBD_OPT_INFO` or `NBD_OPT_GO` this indicates that the
export referred to within the option is either non-existent
or requires TLS; the client MAY therefore choose to issue
an `NBD_OPT_STARTTLS`, MAY terminate the session (if
for instance it does not support TLS or does not have
appropriate credentials for this server), or MAY continue
in another manner without TLS, for instance by querying
or using other exports.

If a client supports TLS, it SHOULD use `NBD_OPT_GO`
(if the server supports it) in place
of `NBD_OPT_EXPORT_NAME`. One reason for this is set out in
the final paragraphs of the sections under 'FORCEDTLS'
and 'SELECTIVETLS': this gives an opportunity for the
server to transmit that an error going into transmission
mode is due to the client's failure to initiate TLS,
and the fact that the client may obtain information about
which exports are TLS-only through `NBD_OPT_INFO`. Another reason is
that the handshake flag `NBD_FLAG_C_NO_ZEROES` can be altered by a
MitM downgrade attack, which can cause a protocol mismatch with
`NBD_OPT_EXPORT_NAME` but not with `NBD_OPT_GO`.

### Security considerations

#### TLS versions

NBD implementations supporting TLS MUST support TLS version 1.2,
SHOULD support any later versions. NBD implementations
MAY support older versions but SHOULD NOT do so by default
(i.e. they SHOULD only be available by a configuration change).
Older versions SHOULD NOT be used where there is a risk of security
problems with those older versions or of a downgrade attack
against TLS versions.

#### Protocol downgrade attacks

A danger inherent in any scheme relying on the negotiation
of whether TLS should be employed is downgrade attacks within
the NBD protocol.

There are two main dangers:

- A Man-in-the-Middle (MitM) hijacks a session and impersonates the
  server (possibly by proxying it) claiming not to support TLS (for
  example, by omitting `NBD_FLAG_FIXED_NEWSTYLE` or changing a
  response to `NBD_OPT_STARTTLS`). In this manner, the client is
  confused into operating in a plain-text manner with the MitM (with
  the session possibly being proxied in plain-text to the server using
  the method below).

- The MitM hijacks a session and impersonates the client (possibly by
  proxying it) claiming not to support TLS (for example, by omitting
  `NBD_FLAG_C_FIXED_NEWSTYLE` or eliding a request for
  `NBD_OPT_STARTTLS`). In this manner the server is confused into
  operating in a plain-text manner with the MitM (with the session
  being possibly proxied to the client with the method above).

With regard to the first, any client that does not wish
to be subject to potential downgrade attack SHOULD ensure
that if a TLS endpoint is specified by the client, it
ensures that TLS is negotiated prior to sending or
requesting sensitive data. To recap, the client MAY send
`NBD_OPT_STARTTLS` at any point during option haggling,
and MAY terminate the session if `NBD_REP_ACK` is not
provided.

With regard to the second, any server that does not wish
to be subject to a potential downgrade attack SHOULD either
used FORCEDTLS mode, or should force TLS on those exports
it is concerned about using SELECTIVE mode and TLS-only
exports. It is not possible to avoid downgrade attacks
on exports which may be served either via TLS or in plain
text unless the client insists on TLS.

## 块大小限制

在传输阶段，多个操作受到最终`NBD_OPT_EXPORT_NAME`或`NBD_OPT_GO`发送的导出大小的限制，以及此处定义的三个块大小限制（最小、首选和最大）的约束。

如果客户端可以遵守服务器的块大小约束（如下所述以及在`NBD_INFO_BLOCK_SIZE`下），它**应该**在握手阶段通过使用`NBD_OPT_GO`（以及`NBD_OPT_INFO`，可选）并附带一个`NBD_INFO_BLOCK_SIZE`信息请求来声明这一点，并且必须使用`NBD_OPT_GO`而不是`NBD_OPT_EXPORT_NAME`(在服务器不支持`NBD_OPT_INFO`或`NBD_OPT_GO`的情况下才使用)。

如果服务器具有非默认的块大小，它**应该**在握手阶段通过对`NBD_OPT_INFO`或`NBD_OPT_GO`的回应中的`NBD_INFO_BLOCK_SIZE`来宣告块大小约束，并且除非已通过线下方式同意了块大小约束，否则**必须**这么做。

Some servers are able to make optimizations, such as opening files
with `O_DIRECT`, if they know that the client will obey a particular
minimum block size, where it must fall back to safer but slower code
if the client might send unaligned requests. For that reason, if a
client issues an `NBD_OPT_GO` including an `NBD_INFO_BLOCK_SIZE`
information request, it MUST abide by the block size constraints it
receives. Clients MAY issue `NBD_OPT_INFO` with `NBD_INFO_BLOCK_SIZE` to
learn the server's constraints without committing to them.

If block size constraints have not been advertised or agreed on
externally, then a server SHOULD support a default minimum block size
of 1, a preferred block size of 2^12 (4,096), and a maximum block size
that is effectively unlimited (0xffffffff, or the export size if that
is smaller), while a client desiring maximum interoperability SHOULD
constrain its requests to a minimum block size of 2^9 (512), and limit
`NBD_CMD_READ` and `NBD_CMD_WRITE` commands to a maximum block size of
2^25 (33,554,432). A server that wants to enforce block sizes other
than the defaults specified here MAY refuse to go into transmission
phase with a client that uses `NBD_OPT_EXPORT_NAME` (via a hard
disconnect) or which uses `NBD_OPT_GO` without requesting
`NBD_INFO_BLOCK_SIZE` (via an error reply of
`NBD_REP_ERR_BLOCK_SIZE_REQD`); but servers SHOULD NOT refuse clients
that do not request sizing information when the server supports
default sizing or where sizing constraints can be agreed on
externally. When allowing clients that did not negotiate sizing via
NBD, a server that enforces stricter block size constraints than the
defaults MUST cleanly error commands that fall outside the constraints
without corrupting data; even so, enforcing constraints in this manner
may limit interoperability.

A client MAY choose to operate as if tighter block size constraints
had been specified (for example, even when the server advertises the
default minimum block size of 1, a client may safely use a minimum
block size of 2^9 (512)).

The minimum block size represents the smallest addressable length and
alignment within the export, although writing to an area that small
may require the server to use a less-efficient read-modify-write
action. If advertised, this value MUST be a power of 2, MUST NOT be
larger than 2^16 (65,536), and MAY be as small as 1 for an export
backed by a regular file, although the values of 2^9 (512) or 2^12
(4,096) are more typical for an export backed by a block device. If a
server advertises a minimum block size, the advertised export size
SHOULD be an integer multiple of that block size, since otherwise, the
client would be unable to access the final few bytes of the export.

The preferred block size represents the minimum size at which aligned
requests will have efficient I/O, avoiding behaviour such as
read-modify-write. If advertised, this MUST be a power of 2 at least
as large as the maximum of the minimum block size and 2^9 (512),
although larger values (such as 4,096, or even the minimum granularity
of a hole) are more typical. The preferred block size MAY be larger
than the export size, in which case the client is unable to utilize
the preferred block size for that export. The server MAY advertise an
export size that is not an integer multiple of the preferred block
size.

The maximum block size represents the maximum length that the server
is willing to handle in one request. If advertised, it MAY be
something other than a power of 2, but MUST be either an integer
multiple of the minimum block size or the value 0xffffffff for no
inherent limit, MUST be at least as large as the smaller of the
preferred block size or export size, and SHOULD be at least 2^20
(1,048,576) if the export is that large. For convenience, the server
MAY advertise a maximum block size that is larger than the export
size, although in that case, the client MUST treat the export size as
the effective maximum block size (as further constrained by a nonzero
offset).

Where a transmission request can have a nonzero *offset* and/or
*length* (such as `NBD_CMD_READ`, `NBD_CMD_WRITE`, or `NBD_CMD_TRIM`),
the client MUST ensure that *offset* and *length* are integer
multiples of any advertised minimum block size, and SHOULD use integer
multiples of any advertised preferred block size where possible. For
those requests, the client MUST NOT use a *length* which, when added to
*offset*, would exceed the export size. Also for NBD*CMD_READ,
NBD_CMD_WRITE, NBD_CMD_CACHE and NBD_CMD_WRITE_ZEROES (except for
when NBD_CMD_FLAG_FAST_ZERO is set), the client MUST NOT use a \_length*
larger than any advertised maximum block size.
The server SHOULD report an `NBD_EINVAL` error if
the client's request is not aligned to advertised minimum block size
boundaries, or is larger than the advertised maximum block size.
Notwithstanding any maximum block size advertised, either the server
or the client MAY initiate a hard disconnect if the payload of an
`NBD_CMD_WRITE` request or `NBD_CMD_READ` reply would be large enough
to be deemed a denial of service attack; however, for maximum
portability, any *length* less than 2^25 (33,554,432) bytes SHOULD NOT
be considered a denial of service attack (even if the advertised
maximum block size is smaller). For all other commands, where the
*length* is not reflected in the payload (such as `NBD_CMD_TRIM` or
`NBD_CMD_WRITE_ZEROES`), a server SHOULD merely fail the command with
an `NBD_EINVAL` error for a client that exceeds the maximum block size,
rather than initiating a hard disconnect.

## Metadata querying

客户端能够查询一系列块的状态通常是非常有帮助的。能够查询的状态的性质在一定程度上依赖于实现。
例如，状态可能表示：

- 在稀疏存储格式中，相关块是否真实存在于导出的后端设备上; 或者

- whether the relevant blocks are 'dirty'; some storage formats and
  operations over such formats express a concept of data dirtiness.
  Whether the operation is block device mirroring, incremental block
  device backup or any other operation with a concept of data
  dirtiness, they all share a need to provide a list of ranges that
  this particular operation treats as dirty.

To provide such classes of information, the NBD protocol has a generic
framework for querying metadata; however, its use must first be
negotiated, and one or more metadata contexts must be selected.

The procedure works as follows:

- First, during negotiation, if the client wishes to query metadata
  during transmission, the client MUST select one or more metadata
  contexts with the `NBD_OPT_SET_META_CONTEXT` command. If needed, the
  client can use `NBD_OPT_LIST_META_CONTEXT` to list contexts that the
  server supports.
- During transmission, a client can then indicate interest in metadata
  for a given region by way of the `NBD_CMD_BLOCK_STATUS` command,
  where *offset* and *length* indicate the area of interest. The
  server MUST then respond with the requested information, for all
  contexts which were selected during negotiation. For every metadata
  context, the server sends one set of extent chunks, where the sizes
  of the extents MUST be less than or equal to the length as specified
  in the request. Each extent comes with a *flags* field, the
  semantics of which are defined by the metadata context.
- A server MUST reply to `NBD_CMD_BLOCK_STATUS` with a structured
  reply of type `NBD_REPLY_TYPE_BLOCK_STATUS`.

A client MUST NOT use `NBD_CMD_BLOCK_STATUS` unless it selected a
nonzero number of metadata contexts during negotiation, and used the
same export name for the subsequent `NBD_OPT_GO` (or
`NBD_OPT_EXPORT_NAME`). Servers SHOULD reply with `NBD_EINVAL` to clients
sending `NBD_CMD_BLOCK_STATUS` without selecting at least one metadata
context.

The reply to the `NBD_CMD_BLOCK_STATUS` request MUST be sent as a
structured reply; this implies that in order to use metadata querying,
structured replies MUST be negotiated first.

Metadata contexts are identified by their names. The name MUST consist
of a namespace, followed by a colon, followed by a leaf-name. The
namespace must consist entirely of printable non-whitespace UTF-8
characters other than colons, and be non-empty. The entire name
(namespace, colon, and leaf-name) MUST follow the restrictions for
strings as laid out earlier in this document.

Namespaces MUST be consist of one of the following:

- `base`, for metadata contexts defined by this document;
- `nbd-server`, for metadata contexts defined by the implementation
  that accompanies this document (none currently);
- `x-*`, where `*` can be replaced by an arbitrary string not
  containing colons, for local experiments. This SHOULD NOT be used
  by metadata contexts that are expected to be widely used.
- A third-party namespace from the list below.

Third-party implementations can register additional namespaces by
simple request to the mailing-list. The following additional
third-party namespaces are currently registered:

- `qemu`, maintained by [qemu.org](https://git.qemu.org/?p=qemu.git;a=blob;f=docs/interop/nbd.txt)

Save in respect of the `base:` namespace described below, this specification
requires no specific semantics of metadata contexts, except that all the
information they provide MUST be representable within the flags field as
defined for `NBD_REPLY_TYPE_BLOCK_STATUS`. Likewise, save in respect of
the `base:` namespace, the syntax of query strings is not specified by this
document, other than the recommendation that the empty leaf-name makes
sense as a wildcard for a client query during `NBD_OPT_LIST_META_CONTEXT`,
but SHOULD NOT select any contexts during `NBD_OPT_SET_META_CONTEXT`.

Server implementations SHOULD ensure the syntax for query strings they
support and semantics for resulting metadata context is documented
similarly to this document.

### The `base:` metadata namespace

This standard defines exactly one metadata context; it is called
`base:allocation`, and it provides information on the basic allocation
status of extents (that is, whether they are allocated at all in a
sparse file context).

The query string within the `base:` metadata context can take one of
two forms:

- `base:` - the server MUST ignore this form during
  `NBD_OPT_SET_META_CONTEXT`, and MUST support this as a wildcard
  during `NBD_OPT_LIST_META_CONTEXT`, in which case the server's reply
  will contain a response for each supported metadata context within
  the `base:` namespace (currently just `base:allocation`, although a
  future revision of the standard might return multiple contexts); or
- `base:[leaf-name]` to select `[leaf-name]` as a context leaf-name
  that might exist within the `base` namespace. If a `[leaf-name]`
  requested by the client is not recognized, the server MUST ignore it
  rather than report an error.

#### `base:allocation` metadata context

The `base:allocation` metadata context is the basic "allocated at all"
metadata context. If an extent is marked with `NBD_STATE_HOLE` at that
context, this means that the given extent is not allocated in the
backend storage, and that writing to the extent MAY result in the
`NBD_ENOSPC` error. This supports sparse file semantics on the server
side. If a server supports the `base:allocation` metadata context,
then writing to an extent which has `NBD_STATE_HOLE` clear MUST NOT
fail with `NBD_ENOSPC` unless for reasons specified in the definition of
another context.

It defines the following flags for the flags field:

- `NBD_STATE_HOLE` (bit 0): if set, the block represents a hole (and
  future writes to that area may cause fragmentation or encounter an
  `NBD_ENOSPC` error); if clear, the block is allocated or the server
  could not otherwise determine its status. Note that the use of
  `NBD_CMD_TRIM` is related to this status, but that the server MAY
  report a hole even where `NBD_CMD_TRIM` has not been requested, and
  also that a server MAY report that the block is allocated even where
  `NBD_CMD_TRIM` has been requested.
- `NBD_STATE_ZERO` (bit 1): if set, the block contents read as all
  zeroes; if clear, the block contents are not known. Note that the
  use of `NBD_CMD_WRITE_ZEROES` is related to this status, but that
  the server MAY report zeroes even where `NBD_CMD_WRITE_ZEROES` has
  not been requested, and also that a server MAY report unknown
  content even where `NBD_CMD_WRITE_ZEROES` has been requested.

It is not an error for a server to report that a region of the export
has both `NBD_STATE_HOLE` set and `NBD_STATE_ZERO` clear. The contents
of such an area are undefined, and a client reading such an area
should make no assumption as to its contents or stability.

For the `base:allocation` context, the remainder of the flags field is
reserved. Servers SHOULD set it to all-zero; clients MUST ignore
unknown flags.

## 常数定义

本节描述了协议中常数（除Magic number外）的值和含义。

当指定标志字段时，它们按网络字节顺序编号。

### 握手阶段

#### Flag fields

##### Handshake flags

这个16位的字段在`INIT_PASSWD`和第一个Magic number之后由服务器发送。

- bit 0, `NBD_FLAG_FIXED_NEWSTYLE`; 如果服务器支持fixed new style协议，则**必须**设置

- bit 1, `NBD_FLAG_NO_ZEROES`; 如果服务器设置，客户端也在回复中的flag中设置了`NBD_FLAG_C_NO_ZEROES`,那么当客户端以`NBD_OPT_EXPORT_NAME`结束协商时，服务器**不得**发送124字节的零。

服务器**不得**设置任何其他标志，并且除非客户端以相应的标志回应，否则**不应**改变行为。在传统式（oldstyle）协商中，服务器**不得**设置这些标志中的任何一个。

在使用TLS时，由于这一阶段容易受到中间人降级攻击（MitM downgrade attacks），NBD协议中不太可能定义额外的能力标志。相反，使用协议选项来协商额外的功能是更好的选择。

##### Client flags

在初始连接之后以及接收到服务器的握手标志后，这个 32 位的字段将被发送。

- bit 0, `NBD_FLAG_C_FIXED_NEWSTYLE`; 如果客户端支持fixed newstyle协议，**应该**设置。服务器**可以**选择对没有设置此位的客户端适用fixed newstyle 协议，但不建议。
- bit 1, `NBD_FLAG_C_NO_ZEROES`; 如果服务器没有设置`NBD_FLAG_NO_ZEROES`，则**不得**设置此标志。如果设置了此标志，当客户端以`NBD_OPT_EXPORT_NAME`结束协商时，服务器**不得**发送124字节的零。

客户端**不得**设置任何其他标志；如果客户端设置了一个未知的标志，或者设置的标志与服务器传输的不匹配，服务器**必须**中断TCP连接。

##### Transmission flags

这个16位的字段在选项协商之后由服务器发送，或者在oldstyle 协商中紧跟 Handshake flags 字段之后立即发送。

许多这样的标志允许服务器向客户端展示它所理解的功能（在这种情况下，它们在下面被记录为 “NBD_FLAG_XXX 暴露功能 YYY”）。
在每种情况下，服务器可以为它支持的功能设置标志。服务器不得为它不支持的功能设置标志。除非该标志被设置，否则客户端不得使用被记录为由标志‘暴露’的功能。

域描述如下:

- bit 0, `NBD_FLAG_HAS_FLAGS`: MUST always be 1.
- bit 1, `NBD_FLAG_READ_ONLY`: 服务器**可以**设置这个标志来向客户端指示导出是只读的（导出可能以服务器无法检测的方式是只读的，例如因为权限问题）。如果这个标志被设置，服务器**必须**对后续对该导出的写操作返回错误。
- bit 2, `NBD_FLAG_SEND_FLUSH`: exposes support for `NBD_CMD_FLUSH`.
- bit 3, `NBD_FLAG_SEND_FUA`: exposes support for `NBD_CMD_FLAG_FUA`.
- bit 4, `NBD_FLAG_ROTATIONAL`: 服务器**可以**将此标志设置为1，以通知客户端该导出具有旋转介质的特性，客户端**可以**根据此标志的设置来调度I/O访问。
- bit 5, `NBD_FLAG_SEND_TRIM`: exposes support for `NBD_CMD_TRIM`.
- bit 6, `NBD_FLAG_SEND_WRITE_ZEROES`: exposes support for `NBD_CMD_WRITE_ZEROES` and `NBD_CMD_FLAG_NO_HOLE`.
- bit 7, `NBD_FLAG_SEND_DF`: 不要分段一个结构化回复(Do not Fragment)。如果`NBD_CMD_READ`请求支持`NBD_CMD_FLAG_DF`标志，服务器**必须**将此传输标志设置为1，并且如果没有协商结构化回复，则必须保持此标志为清除状态。除非设置了此传输标志，否则客户端不得设置`NBD_CMD_FLAG_DF`请求标志。

- bit 8, `NBD_FLAG_CAN_MULTI_CONN`: 表明服务器完全没有使用缓存，或者它使用的缓存在给定设备的所有连接之间共享。特别是，如果存在这个标志，那么当服务器向客户端发送对该命令的回复时，`NBD_CMD_FLUSH`和`NBD_CMD_FLAG_FUA`的效果**必须**在所有连接中可见。在没有这个标志的情况下，客户端**不应该**在多于一个连接上复用它们的命令到导出。
- bit 9, `NBD_FLAG_SEND_RESIZE`: defined by the experimental `RESIZE`
  [extension](https://github.com/NetworkBlockDevice/nbd/blob/extension-resize/doc/proto.md).
- bit 10, `NBD_FLAG_SEND_CACHE`: 标记服务器理解`NBD_CMD_CACHE`命令；然而，请注意，存在一些服务器实现支持该命令但没有设置这一位，反之，设置了这一位也不保证命令一定会成功或产生影响。
- bit 11, `NBD_FLAG_SEND_FAST_ZERO`: 允许客户端检测`NBD_CMD_WRITE_ZEROES`是否比相应的写操作更快。如果`NBD_CMD_WRITE_ZEROES`请求 支持`NBD_CMD_FLAG_FAST_ZERO`标志，服务器**必须**将此传输标志设置为1；如果没有设置`NBD_FLAG_SEND_WRITE_ZEROES`，服务器**必须**将此传输标志设置为0。服务器也**可以**设置此传输标志,即使服务器**可能**对设置了`NBD_CMD_FLAG_FAST_ZERO`的请求全部返回`NBD_ENOTSUP`错误（例如，如果服务器无法快速确定特定的写零请求是否会比常规写更快）。除非设置了此传输标志，否则客户端不得设置`NBD_CMD_FLAG_FAST_ZERO`请求标志。


服务的**可以**忽略未知的标志

#### Option types

下面的值用于在newstyle协议中交换选项("option")

- `NBD_OPT_EXPORT_NAME` (1)

  选择客户端希望使用的导出，结束选项交换，并进入传输阶段。

  Data: String, name of the export, as free-form text.
  名称的长度由选项头部确定。如果选定的导出不存在或者选定的导出的要求没有得到满足（例如，客户端没有为服务器要求TLS的导出启动TLS），服务器必须终止会话。

  一个特殊的“空”名称（即，长度字段为零且没有指定名称），是为“默认”导出保留的，用在明确指定导出名称无意义的情况下。

  这是在非newstyle协商中唯一有效的选项。希望使用任何其他选项的服务器必须支持固定新式。

  这个选项的一个主要问题是它不支持在出现问题时向客户端返回错误消息。为了解决这个问题，引入了`NBD_OPT_GO`（见下文）。

  因此，客户端应优先使用`NBD_OPT_GO`而不是`NBD_OPT_EXPORT_NAME`，但如果不支持`NBD_OPT_GO`（不回退将阻止它连接到旧服务器），应回退到`NBD_OPT_EXPORT_NAME`。

- `NBD_OPT_ABORT` (2)

  客户端希望中止协商并终止会话。服务器必须用`NBD_REP_ACK`回复。

  客户端**不应该**随选项发送任何额外的数据；然而，服务器**应该**忽略客户端发送的任何数据，而不是因为请求无效而拒绝它。

  此文档的早期版本在服务器是否应该对`NBD_OPT_ABORT`发送回复上并不清楚。因此，客户端**应该**优雅地处理服务器在收到一个`NBD_OPT_ABORT`后关闭连接且没有发送回复的情况。同样，服务器**应该**优雅地处理客户端发送一个`NBD_OPT_ABORT`并在不等待回复的情况下关闭连接。

- `NBD_OPT_LIST` (3)

  返回零个或多个 `NBD_REP_SERVER` 回复，每个导出块设备一个，随后返回 `NBD_REP_ACK` 或错误（例如 `NBD_REP_ERR_SHUTDOWN`）。如果尚未协商 TLS，服务器在 SELECTIVETLS 模式下运行，且相关条目是纯 TLS 导出，则服务器可以省略此列表中的条目。

  客户端**不得**发送任何带有该选项的附加数据，服务器**应该**拒绝包含`NBD_REP_ERR_INVALID`数据的请求。


- `NBD_OPT_PEEK_EXPORT` (4)

  Was defined by the (withdrawn) experimental `PEEK_EXPORT` extension;
  not in use.

- `NBD_OPT_STARTTLS` (5)

  The client wishes to initiate TLS.

  The client MUST NOT send any additional data with the option. The
  server MUST either reply with `NBD_REP_ACK` after which point the
  connection is upgraded to TLS, or an error reply explicitly
  permitted by this document (for example, `NBD_REP_ERR_INVALID` if
  the client included data).

  When this command succeeds, the server MUST NOT preserve any
  negotiation state (such as a request for
  `NBD_OPT_STRUCTURED_REPLY`, or metadata contexts from
  `NBD_OPT_SET_META_CONTEXT`) issued before this command. A client
  SHOULD defer all stateful option requests until after it
  determines whether encryption is available.

  See the section on TLS above for further details.

- `NBD_OPT_INFO` (6) and `NBD_OPT_GO` (7)

  这两个选项的请求和回复格式完全相同。唯一不同的是，在成功回复 `NBD_OPT_GO`（即一个或多个 `NBD_REP_INFO`，然后一个 `NBD_REP_ACK`）后，会立即进入传输模式。因此，这些命令共享共同的文档。

  `NBD_OPT_INFO`:客户机希望获取带有给定名称的导出的详细信息，以便在传输阶段使用，但还不想进入传输阶段。成功后，该选项将提供比 `NBD_OPT_LIST` 更多的详细信息，但仅限于单个出口名称。

  `NBD_OPT_GO`: 客户机希望终止握手阶段并进入传输阶段。该客户机可以在发出 `NBD_OPT_INFO` 命令后发出该命令，也可以在没有发出 `NBD_OPT_INFO` 命令的情况下发出该命令。因此，`NBD_OPT_GO`可用作 `NBD_OPT_EXPORT_NAME`的改进版本，它能够返回错误。

  Data (both commands):

  - 32 bits, length of name (unsigned);  不得大于 option data 长度 - 6
  - String: name of the export
  - 16 bits, number of information requests
  - 16 bits x n , `NBD_INFO`信息请求列表

  客户**可以**在信息请求列表中列出一个或多个所需的特定信息项目，也可指定一个空列表。客户端**不得**在列表中多次包含任何相同信息请求。
  服务器**必须**忽略它不理解的任何信息请求。
  服务器**可以**按任何顺序回复信息请求。
  服务器**可以**忽略它因策略原因（除 `NBD_INFO_EXPORT`）而不想提供的信息请求。
  同样，如果没有提供客户机请求的信息，客户机也**可以**拒绝协商。
  服务器**可以**回复客户端未请求的INFO，但服务器**不得**假定客户机理解并遵守此类信息请求，只有客户端明确请求的才会被保证遵守。
  客户端**必须**忽略其不理解的信息回复。

  如果没有指定名称（即提供的字符串长度为零），这将指定默认的导出（如果有），与 `NBD_OPT_EXPORT_NAME`一样。

  服务器将回复若干个 `NBD_REP_INFO`（如果报错，则回复数量为零；如果成功，则至少回复一个），然后以最后的错误回复或成功声明结束信息列表，如下所示：

  - `NBD_REP_ACK`: 服务器接受所选的导出，并已完成信息提供。在这种情况下，服务器必须发送至少一个信息类型为 `NBD_INFO_EXPORT` 的 `NBD_REP_INFO` 。
  - `NBD_REP_ERR_UNKNOWN`: 该服务器上不存在所选的导出。在这种情况下，服务器**不应**发送`NBD_REP_INFO`回复。
  - `NBD_REP_ERR_TLS_REQD`: 服务器要求客户端在透露有关此导出的任何进一步细节之前启动 TLS。在这种情况下，FORCEDTLS 服务器**不得**发送 `NBD_REP_INFO` 回复。
    但如果是纯 TLS 导出，   SELECTIVETLS 服务器则**可以**发送 `NBD_REP_INFO` 回复。
  - `NBD_REP_ERR_BLOCK_SIZE_REQD`: 服务器要求客户端在进入传输阶段之前使用 `NBD_INFO_BLOCK_SIZE`请求块大小限制，因为服务器将使用非默认块大小限制。
    如果使用 `NBD_INFO_BLOCK_SIZE` 与 `NBD_OPT_INFO` 或 `NBD_OPT_GO` 请求 请求过块大小限制，服务器**不得**发送此错误。
    如果服务器使用的是默认块大小限制或协议外协商的块大小限制，则**不应**发送此错误。
    发送 `NBD_REP_ERR_BLOCK_SIZE_REQD` 错误的服务器**应该**确保首先发送 `NBD_INFO_BLOCK_SIZE` 信息回复，以帮助避免可能不必要的往返。

  此外，如果 TLS 尚未启动，服务器可以用 `NBD_REP_ERR_TLS_REQD`（而不是 `NBD_REP_ERR_UNKNOWN`）来回复未知导出的请求。这样，未启动 TLS 的客户端就无法枚举导出设备。选择以这种方式隐藏未知出口的 SELECTIVETLS 服务器**不应该**为仅 TLS 出口发送 `NBD_REP_INFO` 回复。

  为了向后兼容，客户端也**应该**准备好通过使用 `NBD_OPT_EXPORT_NAME`来处理 `NBD_REP_ERR_UNSUP`。

  其它错误（如 `NBD_REP_ERR_SHUTDOWN`）也是可能的，这在本文档的其它地方也是允许的，而且对前面的 `NBD_REP_INFO`次数没有限制。

  如果在一个成功的 `NBD_OPT_INFO`（即以最终的 `NBD_REP_ACK`结束回复）和一个具有相同参数（包括所请求的信息项目列表）的 `NBD_OPT_GO`之间没有任何中间选项请求，那么服务器必须以相同的信息集（如 `NBD_INFO_EXPORT` 回复中的传输标志）进行回复，尽管中间 `NBD_REP_INFO` 消息的排序可能不同。 否则，由于中间的选项请求或使用了不同的参数，服务器可能会在成功的响应中发送不同的数据，和/或可能会使第二个请求失败。

  对 `NBD_OPT_GO` 的回复与对 `NBD_OPT_INFO` 的回复相同，但如果回复表示成功（即以 `NBD_REP_ACK` 结束），客户端和服务器都会立即进入传输阶段。无论客户端是否协商了 `NBD_FLAG_C_NO_ZEROES` 标记，服务器都**不得**在 `NBD_REP_ACK` 数据后发送任何零填充字节。除非服务器的最终回复显示出错，否则客户端**不得**再发送选项请求。

- `NBD_OPT_GO` (7)

  See above under `NBD_OPT_INFO`.

- `NBD_OPT_STRUCTURED_REPLY` (8)

  客户端希望在传输阶段使用结构化回复。客户机不得发送任何带有该选项的附加数据，服务器**应该**拒绝包含`NBD_REP_ERR_INVALID`数据的请求。

  服务器将回复以下内容，或本文档其他地方允许的错误：

  - `NBD_REP_ACK`: 已协商结构化回复；服务器**必须**对 `NBD_CMD_READ` 传输请求使用结构化回复。现在可以协商其他需要结构化回复的扩展。
  - 为了向后兼容，客户机应准备好同时处理 `NBD_REP_ERR_UNSUP`；在这种情况下，将不会发送结构化回复。

  根据设想，未来的扩展将增加其他新请求，这些请求可能需要在回复中包含数据有效载荷。支持此类扩展的服务器不应在客户机协商结构化回复之前公布这些扩展；客户机在未启用 `NBD_OPT_STRUCTURED_REPLY` 扩展之前不得使用这些扩展。

  如果客户机在此选项之后请求 `NBD_OPT_STARTTLS`，它必须重新协商结构化回复和它希望使用的任何其他依赖扩展。

- `NBD_OPT_LIST_META_CONTEXT` (9)

  Return a list of `NBD_REP_META_CONTEXT` replies, one per context,
  followed by an `NBD_REP_ACK` or an error.

  This option SHOULD NOT be requested unless structured replies have
  been negotiated first. If a client attempts to do so, a server
  MAY send `NBD_REP_ERR_INVALID`.

  Data:

  - 32 bits, length of export name.
  - String, name of export for which we wish to list metadata
    contexts.
  - 32 bits, number of queries
  - Zero or more queries, each being:
    - 32 bits, length of query.
    - String, query to list a subset of the available metadata
      contexts. The syntax of this query is
      implementation-defined, except that it MUST start with a
      namespace and a colon.

  For details on the query string, see the "Metadata querying"
  section; note that a namespace may document that a different set
  of queries are valid for `NBD_OPT_LIST_META_CONTEXT` than for
  `NBD_OPT_SET_META_CONTEXT`, such as when using an empty leaf-name
  for wildcarding.

  If the option request is syntactically invalid (such as a query
  length that would require reading beyond the original length given
  in the option header), the server MUST fail the request with
  `NBD_REP_ERR_INVALID`. For requests that are semantically invalid
  (such as lacking the required colon that delimits the namespace,
  or using a leaf name that is invalid for a known namespace), the
  server MAY fail the request with `NBD_REP_ERR_INVALID`. However,
  the server MUST ignore query strings belonging to an unknown
  namespace. If none of the query strings find any metadata
  contexts, the server MUST send a single reply of type
  `NBD_REP_ACK`.

  The server MUST reply with a list of zero or more
  `NBD_REP_META_CONTEXT` replies, followed by either a final
  `NBD_REP_ACK` on success or by an error (for instance
  `NBD_REP_ERR_UNSUP` if the option is not supported). If an error
  is returned, the client MUST disregard any context replies that
  may have been sent.

  If zero queries are sent, then the server MUST return all the
  metadata contexts that are available to the client to select on
  the given export. However, this list may include wildcards that
  require a further `NBD_OPT_LIST_META_CONTEXT` with the wildcard as
  a query, rather than an actual context that is appropriate as a
  query to `NBD_OPT_SET_META_CONTEXT`, as set out below. In this
  case, the server SHOULD NOT fail with `NBD_REP_ERR_TOO_BIG`.

  If one or more queries are sent, then the server MUST return those
  metadata contexts that are available to the client to select on
  the given export with `NBD_OPT_SET_META_CONTEXT`, and which match
  one or more of the queries given. The support of wildcarding
  within the leaf-name portion of the query string is dependent upon
  the namespace. The server MAY send contexts in a different order
  than in the client's query. In this case, the server MAY fail
  with `NBD_REP_ERR_TOO_BIG` if too many queries are requested.

  In either case, however, for any given namespace the server MAY,
  instead of exhaustively listing every matching context available
  to select (or every context available to select where no query is
  given), send sufficient context records back to allow a client
  with knowledge of the namespace to select any context. This may
  be helpful where a client can construct algorithmic queries. For
  instance, a client might reply simply with the namespace with no
  leaf-name (e.g. 'x-FooBar:') or with a range of values (e.g.
  'x-ModifiedDate:20160310-20161214'). The semantics of such a reply
  are a matter for the definition of the namespace. However each
  namespace returned MUST begin with the relevant namespace,
  followed by a colon, and then other UTF-8 characters, with the
  entire string following the restrictions for strings set out
  earlier in this document.

  The metadata context ID in these replies is reserved and SHOULD be
  set to zero; clients MUST disregard it.

- `NBD_OPT_SET_META_CONTEXT` (10)

  Change the set of active metadata contexts. Issuing this command
  replaces all previously-set metadata contexts (including when this
  command fails); clients must ensure that all metadata contexts
  they are interested in are selected with the final query that they
  sent.

  This option MUST NOT be requested unless structured replies have
  been negotiated first. If a client attempts to do so, a server
  SHOULD send `NBD_REP_ERR_INVALID`.

  A client MUST NOT send `NBD_CMD_BLOCK_STATUS` unless within the
  negotiation phase it sent `NBD_OPT_SET_META_CONTEXT` at least
  once, and where the final time it was sent, it referred to the
  same export name that was ultimately selected for transmission
  phase with no intervening `NBD_OPT_STARTTLS`, and where the server
  responded by returning least one metadata context without error.

  Data:

  - 32 bits, length of export name.
  - String, name of export for which we wish to list metadata
    contexts.
  - 32 bits, number of queries
  - Zero or more queries, each being:
    - 32 bits, length of query
    - String, query to select metadata contexts. The syntax of this
      query is implementation-defined, except that it MUST start with a
      namespace and a colon.

  If zero queries are sent, the server MUST select no metadata
  contexts.

  The server MAY return `NBD_REP_ERR_TOO_BIG` if a request seeks to
  select too many contexts. Otherwise the server MUST reply with a
  number of `NBD_REP_META_CONTEXT` replies, one for each selected
  metadata context, each with a unique metadata context ID, followed
  by `NBD_REP_ACK`. The server MAY ignore queries that do not select
  a single metadata context, and MAY return selected contexts in a
  different order than in the client's request. The metadata
  context ID is transient and may vary across calls to
  `NBD_OPT_SET_META_CONTEXT`; clients MUST therefore treat the ID as
  an opaque value and not (for instance) cache it between
  connections. It is not an error if a `NBD_OPT_SET_META_CONTEXT`
  option does not select any metadata context, provided the client
  then does not attempt to issue `NBD_CMD_BLOCK_STATUS` commands.

#### Option reply types

These values are used in the "reply type" field, sent by the server during option haggling in the fixed newstyle negotiation.

- `NBD_REP_ACK` (1)

  Will be sent by the server when it accepts the option and no further information is available, or when sending data related to the option (in the case of `NBD_OPT_LIST`) has finished. No data.

- `NBD_REP_SERVER` (2)

  A description of an export. Data:

  - 32 bits, length of name (unsigned); MUST be no larger than the reply packet header length - 4
  - String, name of the export, as expected by `NBD_OPT_EXPORT_NAME`, `NBD_OPT_INFO`, or `NBD_OPT_GO`
  - If length of name < (reply packet header length - 4), then the rest of the data contains some implementation-specific details about the export. This is not currently implemented, but future versions of nbd-server may send along some details about the export. Therefore, unless explicitly documented otherwise by a particular client request, this field is defined to be a string suitable for direct display to a human being.

- `NBD_REP_INFO` (3)

  A detailed description about an aspect of an export. The response to `NBD_OPT_INFO` and `NBD_OPT_GO` includes zero or more of these messages prior to a final error reply, or at least one before an `NBD_REP_ACK` reply indicating success. The server MUST send an `NBD_INFO_EXPORT` information type at some point before sending an `NBD_REP_ACK`, so that `NBD_OPT_GO` can provide a superset of the information given in response to `NBD_OPT_EXPORT_NAME`; all other information types are optional. A particular information type SHOULD only appear once for a given export unless documented otherwise.

  A client MUST NOT rely on any particular ordering amongst the `NBD_OPT_INFO` replies, and MUST ignore information types that it does not recognize.

  The acceptable values for the header *length* field are determined by the information type, and includes the 2 bytes for the type designator, in the following general layout:

  - 16 bits, information type (e.g. `NBD_INFO_EXPORT`)
  - *length - 2* bytes, information payload

  The following information types are defined:

  - `NBD_INFO_EXPORT` (0)

    Mandatory information before a successful completion of
    `NBD_OPT_INFO` or `NBD_OPT_GO`. Describes the same information
    that is sent in response to the older `NBD_OPT_EXPORT_NAME`,
    except that there are no trailing zeroes whether or not
    `NBD_FLAG_C_NO_ZEROES` was negotiated. *length* MUST be 12, and
    the reply payload is interpreted as follows:

    - 16 bits, `NBD_INFO_EXPORT`
    - 64 bits, size of the export in bytes (unsigned)
    - 16 bits, transmission flags

  - `NBD_INFO_NAME` (1)

    Represents the server's canonical name of the export. The name
    MAY differ from the name presented in the client's option
    request, and the information item MAY be omitted if the client
    option request already used the canonical name. This
    information type represents the same name that would appear in
    the name portion of an `NBD_REP_SERVER` in response to
    `NBD_OPT_LIST`. The *length* MUST be at least 2, and the reply
    payload is interpreted as:

    - 16 bits, `NBD_INFO_NAME`
    - String: name of the export, *length - 2* bytes

  - `NBD_INFO_DESCRIPTION` (2)

    A description of the export, suitable for direct display to the
    human being. This information type represents the same optional
    description that may appear after the name portion of an
    `NBD_REP_SERVER` in response to `NBD_OPT_LIST`. The *length*
    MUST be at least 2, and the reply payload is interpreted as:

    - 16 bits, `NBD_INFO_DESCRIPTION`
    - String: description of the export, *length - 2* bytes

  - `NBD_INFO_BLOCK_SIZE` (3)

    Represents the server's advertised block size constraints; see the
    "Block size constraints" section for more details on what these
    values represent, and on constraints on their values. The server
    MUST send this info if it is requested and it intends to enforce
    block size constraints other than the defaults. After
    sending this information in response to an `NBD_OPT_GO` in which
    the client specifically requested `NBD_INFO_BLOCK_SIZE`, the server
    can legitimately assume that any client that continues the session
    will support the block size constraints supplied (note that this
    assumption cannot be made solely on the basis of an `NBD_OPT_INFO`
    with an `NBD_INFO_BLOCK_SIZE` request, or an `NBD_OPT_GO` without
    an explicit `NBD_INFO_BLOCK_SIZE` request). The *length* MUST be 14,
    and the reply payload is interpreted as:

    - 16 bits, `NBD_INFO_BLOCK_SIZE`
    - 32 bits, minimum block size
    - 32 bits, preferred block size
    - 32 bits, maximum block size

- `NBD_REP_META_CONTEXT` (4)

  A description of a metadata context. Data:

  - 32 bits, NBD metadata context ID.
  - String, name of the metadata context. This is not required to be
    a human-readable string, but it MUST be valid UTF-8 data.

There are a number of error reply types, all of which are denoted by
having bit 31 set. All error replies MAY have some data set, in which
case that data is an error message string suitable for display to the user.

- `NBD_REP_ERR_UNSUP` (2^31 + 1)

  The option sent by the client is unknown by this server
  implementation (e.g., because the server is too old, or from another
  source).

- `NBD_REP_ERR_POLICY` (2^31 + 2)

  The option sent by the client is known by this server and
  syntactically valid, but server-side policy forbids the server to
  allow the option (e.g., the client sent `NBD_OPT_LIST` but server
  configuration has that disabled)

- `NBD_REP_ERR_INVALID` (2^31 + 3)

  The option sent by the client is known by this server, but was
  determined by the server to be syntactically or semantically
  invalid. For instance, the client sent an `NBD_OPT_LIST` with
  nonzero data length, or the client sent a second
  `NBD_OPT_STARTTLS` after TLS was already negotiated.

- `NBD_REP_ERR_PLATFORM` (2^31 + 4)

  The option sent by the client is not supported on the platform on
  which the server is running, or requires compile-time options that
  were disabled, e.g., upon trying to use TLS.

- `NBD_REP_ERR_TLS_REQD` (2^31 + 5)

  The server is unwilling to continue negotiation unless TLS is
  initiated first. In the case of `NBD_OPT_INFO` and `NBD_OPT_GO`
  this unwillingness MAY (depending on the TLS mode) be limited
  to the export in question. See the section on TLS above for
  further details.

- `NBD_REP_ERR_UNKNOWN` (2^31 + 6)

  The requested export is not available.

- `NBD_REP_ERR_SHUTDOWN` (2^31 + 7)

  The server is unwilling to continue negotiation as it is in the
  process of being shut down.

- `NBD_REP_ERR_BLOCK_SIZE_REQD` (2^31 + 8)

  The server is unwilling to enter transmission phase for a given
  export unless the client first acknowledges (via
  `NBD_INFO_BLOCK_SIZE`) that it will obey non-default block sizing
  requirements.

- `NBD_REP_ERR_TOO_BIG` (2^31 + 9)

  The request or the reply is too large to process.

### Transmission phase

#### Flag fields

##### Command flags

这个 16 位的字段由客户端随每个请求发送，并为服务器执行命令提供额外信息。具体每个标志如何影响特定命令的详细信息，请参考下面的“Request types”部分。客户端**不得**设置未在特定命令中记录的命令标志位；并且标志是否有效可能取决于握手阶段的协商。

- bit 0, `NBD_CMD_FLAG_FUA`; This flag is valid for all commands, provided `NBD_FLAG_SEND_FUA` has been negotiated, in which case the server MUST accept all commands with this bit set (even by ignoring the bit). The client SHOULD NOT set this bit unless the command has the potential of writing data (current commands are `NBD_CMD_WRITE`, `NBD_CMD_WRITE_ZEROES` and `NBD_CMD_TRIM`), however note that existing clients are known to set this bit on other commands. Subject to that, and provided `NBD_FLAG_SEND_FUA` is negotiated, the client MAY set this bit on all, no or some commands as it wishes (see the section on Ordering of messages and writes for details). If the server receives a command with `NBD_CMD_FLAG_FUA` set it MUST NOT send its reply to that command until all write operations (if any) associated with that command have been completed and persisted to non-volatile storage. If the command does not in fact write data (for instance on an `NBD_CMD_TRIM` in a situation where the command as a whole is ignored), the server MAY ignore this bit being set on such a command.

- bit 1, `NBD_CMD_FLAG_NO_HOLE`; valid during `NBD_CMD_WRITE_ZEROES`.
  SHOULD be set to 1 if the client wants to ensure that the server does
  not create a hole. The client MAY send `NBD_CMD_FLAG_NO_HOLE` even
  if `NBD_FLAG_SEND_TRIM` was not set in the transmission flags field.
  The server MUST support the use of this flag if it advertises
  `NBD_FLAG_SEND_WRITE_ZEROES`.

- bit 2, `NBD_CMD_FLAG_DF`; the "don't fragment" flag, valid during
  `NBD_CMD_READ`. SHOULD be set to 1 if the client requires the
  server to send at most one content chunk in reply. MUST NOT be set
  unless the transmission flags include `NBD_FLAG_SEND_DF`. Use of
  this flag MAY trigger an `NBD_EOVERFLOW` error chunk, if the request
  length is too large.

- bit 3, `NBD_CMD_FLAG_REQ_ONE`; valid during
  `NBD_CMD_BLOCK_STATUS`. If set, the client is interested in only one
  extent per metadata context. If this flag is present, the server
  MUST NOT send metadata on more than one extent in the reply. Client
  implementors should note that using this flag on multiple contiguous
  requests is likely to be inefficient.

- bit 4, `NBD_CMD_FLAG_FAST_ZERO`; valid during
  `NBD_CMD_WRITE_ZEROES`. If set, but the server cannot perform the
  write zeroes any faster than it would for an equivalent
  `NBD_CMD_WRITE`, then the server MUST fail quickly with an error of
  `NBD_ENOTSUP`. The client MUST NOT set this unless the server advertised
  `NBD_FLAG_SEND_FAST_ZERO`.

##### Structured reply flags

This field of 16 bits is sent by the server as part of every
structured reply.

- bit 0, `NBD_REPLY_FLAG_DONE`; the server MUST clear this bit if
  more structured reply chunks will be sent for the same client
  request, and MUST set this bit if this is the final reply. This
  bit MUST always be set for the `NBD_REPLY_TYPE_NONE` chunk,
  although any other chunk type can also be used as the final
  chunk.

The server MUST NOT set any other flags without first negotiating
the extension with the client, unless the client can usefully
react to the response without interpreting the flag (for instance
if the flag is some form of hint). Clients MUST ignore
unrecognized flags.

#### Structured reply types

These values are used in the "type" field of a structured reply.
Some chunk types can additionally be categorized by role, such as
*error chunks* or *content chunks*. Each type determines how to
interpret the "length" bytes of payload. If the client receives
an unknown or unexpected type, other than an *error chunk*, it
MUST initiate a hard disconnect.

- `NBD_REPLY_TYPE_NONE` (0)

  *length* MUST be 0 (and the payload field omitted). This chunk
  type MUST always be used with the `NBD_REPLY_FLAG_DONE` bit set
  (that is, it may appear at most once in a structured reply, and
  is only useful as the final reply chunk). If no earlier error
  chunks were sent, then this type implies that the overall client
  request is successful. Valid as a reply to any request.

- `NBD_REPLY_TYPE_OFFSET_DATA` (1)

  This chunk type is in the content chunk category. *length* MUST be
  at least 9. It represents the contents of *length - 8* bytes of the
  file, starting at the absolute *offset* from the start of the
  export. The data MUST lie within the bounds of the original offset
  and length of the client's request, and MUST NOT overlap with the
  bounds of any earlier content chunk or error chunk in the same
  reply. This chunk MAY be used more than once in a reply, unless the
  `NBD_CMD_FLAG_DF` flag was set. Valid as a reply to `NBD_CMD_READ`.

  The payload is structured as:

  64 bits: offset (unsigned)  
  *length - 8* bytes: data

- `NBD_REPLY_TYPE_OFFSET_HOLE` (2)

  This chunk type is in the content chunk category. *length* MUST be
  exactly 12. It represents that the contents of *hole size* bytes,
  starting at the absolute *offset* from the start of the export, read
  as all zeroes. The hole MUST lie within the bounds of the original
  offset and length of the client's request, and MUST NOT overlap with
  the bounds of any earlier content chunk or error chunk in the same
  reply. This chunk MAY be used more than once in a reply, unless the
  `NBD_CMD_FLAG_DF` flag was set. Valid as a reply to `NBD_CMD_READ`.

  The payload is structured as:

  64 bits: offset (unsigned)  
  32 bits: hole size (unsigned, MUST be nonzero)

- `NBD_REPLY_TYPE_BLOCK_STATUS` (5)

  *length* MUST be 4 + (a positive integer multiple of 8). This reply
  represents a series of consecutive block descriptors where the sum
  of the length fields within the descriptors is subject to further
  constraints documented below. This chunk type MUST appear
  exactly once per metadata ID in a structured reply.

  The payload starts with:

  32 bits, metadata context ID

  and is followed by a list of one or more descriptors, each with this
  layout:

  32 bits, length of the extent to which the status below
  applies (unsigned, MUST be nonzero)  
  32 bits, status flags

  If the client used the `NBD_CMD_FLAG_REQ_ONE` flag in the request,
  then every reply chunk MUST contain exactly one descriptor, and that
  descriptor MUST NOT exceed the *length* of the original request. If
  the client did not use the flag, and the server replies with N
  extents, then the sum of the *length* fields of the first N-1
  extents (if any) MUST be less than the requested length, while the
  *length* of the final extent MAY result in a sum larger than the
  original requested length, if the server has that information anyway
  as a side effect of reporting the status of the requested region.

  Even if the client did not use the `NBD_CMD_FLAG_REQ_ONE` flag in
  its request, the server MAY return fewer descriptors in the reply
  than would be required to fully specify the whole range of requested
  information to the client, if looking up the information would be
  too resource-intensive for the server, so long as at least one
  extent is returned. Servers should however be aware that most
  clients implementations will then simply ask for the next extent
  instead.

All error chunk types have bit 15 set, and begin with the same
*error*, *message length*, and optional *message* fields as
`NBD_REPLY_TYPE_ERROR`. If nonzero, *message length* indicates
that an optional error string message appears next, suitable for
display to a human user. The header *length* then covers any
remaining structured fields at the end.

- `NBD_REPLY_TYPE_ERROR` (2^15 + 1)

  This chunk type is in the error chunk category. *length* MUST
  be at least 6. This chunk represents that an error occurred,
  and the client MAY NOT make any assumptions about partial
  success. This type SHOULD NOT be used more than once in a
  structured reply. Valid as a reply to any request.

  The payload is structured as:

  32 bits: error (MUST be nonzero)  
  16 bits: message length (no more than header *length* - 6)  
  *message length* bytes: optional string suitable for
  direct display to a human being

- `NBD_REPLY_TYPE_ERROR_OFFSET` (2^15 + 2)

  This chunk type is in the error chunk category. *length* MUST
  be at least 14. This reply represents that an error occurred at
  a given offset, which MUST lie within the original offset and
  length of the request; the client can use this offset to
  determine if request had any partial success. This chunk type
  MAY appear multiple times in a structured reply, although the
  same offset SHOULD NOT be repeated. Likewise, if content chunks
  were sent earlier in the structured reply, the server SHOULD NOT
  send multiple distinct offsets that lie within the bounds of a
  single content chunk. Valid as a reply to `NBD_CMD_READ`,
  `NBD_CMD_WRITE`, `NBD_CMD_TRIM`, and `NBD_CMD_BLOCK_STATUS`.

  The payload is structured as:

  32 bits: error (MUST be nonzero)  
  16 bits: message length (no more than header *length* - 14)  
  *message length* bytes: optional string suitable for
  direct display to a human being  
  64 bits: offset (unsigned)

If the client receives an unknown or unexpected type with bit 15
set, it MUST consider the current reply as errored, but MAY
continue transmission unless it detects that *message length* is
too large to fit within the *length* specified by the header. For
all other messages with unknown or unexpected type or inconsistent
contents, the client MUST initiate a hard disconnect.

#### Request types

The following request types exist:

- `NBD_CMD_READ` (0)

  A read request. Length and offset define the data to be read. The
  server MUST reply with either a simple reply or a structured
  reply, according to whether the structured replies have been
  negotiated using `NBD_OPT_STRUCTURED_REPLY`. The client SHOULD NOT
  request a read length of 0; the behavior of a server on such a
  request is unspecified although the server SHOULD NOT disconnect.

  *Simple replies*

  If structured replies were not negotiated, then a read request
  MUST always be answered by a simple reply, as documented above
  (using magic 0x67446698 `NBD_SIMPLE_REPLY_MAGIC`, and containing
  length bytes of data according to the client's request).

  If an error occurs, the server SHOULD set the appropriate error code
  in the error field. The server MAY then initiate a hard disconnect.
  If it chooses not to, it MUST NOT send any payload for this request.

  If an error occurs while reading after the server has already sent
  out the reply header with an error field set to zero (i.e.,
  signalling no error), the server MUST immediately initiate a
  hard disconnect; it MUST NOT send any further data to the client.

  *Structured replies*

  If structured replies are negotiated, then a read request MUST
  result in a structured reply with one or more chunks (each using
  magic 0x668e33ef `NBD_STRUCTURED_REPLY_MAGIC`), where the final
  chunk has the flag `NBD_REPLY_FLAG_DONE`, and with the following
  additional constraints.

  The server MAY split the reply into any number of content chunks;
  each chunk MUST describe at least one byte, although to minimize
  overhead, the server SHOULD use chunks with lengths and offsets as
  an integer multiple of 512 bytes, where possible (the first and
  last chunk of an unaligned read being the most obvious places for
  an exception). The server MUST NOT send content chunks that
  overlap with any earlier content or error chunk, and MUST NOT send
  chunks that describe data outside the offset and length of the
  request, but MAY send the content chunks in any order (the client
  MUST reassemble content chunks into the correct order), and MAY
  send additional content chunks even after reporting an error chunk.
  Note that a request for more than 2^32 - 8 bytes MUST be split
  into at least two chunks, so as not to overflow the length field
  of a reply while still allowing space for the offset of each
  chunk. When no error is detected, the server MUST send enough
  data chunks to cover the entire region described by the offset and
  length of the client's request.

  To minimize traffic, the server MAY use a content or error chunk
  as the final chunk by setting the `NBD_REPLY_FLAG_DONE` flag, but
  MUST NOT do so for a content chunk if it would still be possible
  to detect an error while transmitting the chunk. The
  `NBD_REPLY_TYPE_NONE` chunk is always acceptable as the final
  chunk.

  If an error is detected, the server MUST still complete the
  transmission of any current chunk (it MUST use padding bytes which
  SHOULD be zero, for any remaining data portion of a chunk with
  type `NBD_REPLY_TYPE_OFFSET_DATA`), but MAY omit further content
  chunks. The server MUST include an error chunk as one of the
  subsequent chunks, but MAY defer the error reporting behind other
  queued chunks. An error chunk of type `NBD_REPLY_TYPE_ERROR`
  implies that the client MAY NOT make any assumptions about
  validity of data chunks (whether sent before or after the error
  chunk), and if used, SHOULD be the only error chunk in the reply.
  On the other hand, an error chunk of type
  `NBD_REPLY_TYPE_ERROR_OFFSET` gives fine-grained information about
  which earlier data chunk(s) encountered a failure; as such, a
  server MAY still usefully follow it with further non-overlapping
  content chunks or with error offsets for other content chunks.
  The server MAY send an error chunk with no corresponding content
  chunk, but MUST ensure that the content chunk is sent first if a
  content and error chunk cover the same offset. Generally, a
  server SHOULD NOT mix errors with offsets with a generic error.
  As long as all errors are accompanied by offsets, the client MAY
  assume that any data chunks with no subsequent error offset are
  valid, that chunks with an overlapping error offset errors are
  valid up until the reported offset, and that portions of the read
  that do not have a corresponding content chunk are not valid.

  A client MAY initiate a hard disconnect if it detects that the server
  has sent invalid chunks (such as overlapping data, or not enough
  data before claiming success).

  In order to avoid the burden of reassembly, the client MAY set the
  `NBD_CMD_FLAG_DF` flag ("don't fragment"). If this flag is set,
  the server MUST send at most one content chunk, although it MAY
  still send multiple chunks (the remaining chunks would be error
  chunks or a final type of `NBD_REPLY_TYPE_NONE`). If the area
  being read contains both data and a hole, the server MUST use
  `NBD_REPLY_TYPE_OFFSET_DATA` with the zeroes explicitly present.
  A server MAY reject a client's request with the error `NBD_EOVERFLOW`
  if the length is too large to send without fragmentation, in which
  case it MUST NOT send a content chunk; however, the server MUST
  support unfragmented reads in which the client's request length
  does not exceed 65,536 bytes.

- `NBD_CMD_WRITE` (1)

  A write request. Length and offset define the location and amount of
  data to be written. The client MUST follow the request header with
  *length* number of bytes to be written to the device. The client
  SHOULD NOT request a write length of 0; the behavior of a server on
  such a request is unspecified although the server SHOULD NOT
  disconnect.

  The server MUST write the data to disk, and then send the reply
  message. The server MAY send the reply message before the data has
  reached permanent storage, unless `NBD_CMD_FLAG_FUA` is in use.

  If an error occurs, the server MUST set the appropriate error code
  in the error field.

- `NBD_CMD_DISC` (2)

  A disconnect request. The server MUST handle all outstanding
  requests, shut down the TLS session (if one is running), and
  close the TCP session. A client MUST NOT send
  anything to the server after sending an `NBD_CMD_DISC` command.

  The values of the length and offset fields in a disconnect request
  MUST be zero.

  There is no reply to an `NBD_CMD_DISC`.

- `NBD_CMD_FLUSH` (3)

  A flush request. The server MUST NOT send a
  successful reply header for this request before all write requests
  for which a reply has already been sent to the client have reached
  permanent storage (using fsync() or similar).

  A client MUST NOT send a flush request unless `NBD_FLAG_SEND_FLUSH`
  was set in the transmission flags field.

  For a flush request, *length* and *offset* are reserved, and MUST be
  set to all-zero.

- `NBD_CMD_TRIM` (4)

  A hint to the server that the data defined by length and offset is
  no longer needed. A server MAY discard *length* bytes starting at
  offset, but is not required to; and MAY round *offset* up and
  *length* down to meet internal alignment constraints so that only
  a portion of the client's request is actually discarded. The
  client SHOULD NOT request a trim length of 0; the behavior of a
  server on such a request is unspecified although the server SHOULD
  NOT disconnect.

  After issuing this command, a client MUST NOT make any assumptions
  about the contents of the export affected by this command, until
  overwriting it again with `NBD_CMD_WRITE` or `NBD_CMD_WRITE_ZEROES`.

  A client MUST NOT send a trim request unless `NBD_FLAG_SEND_TRIM`
  was set in the transmission flags field.

- `NBD_CMD_CACHE` (5)

  A cache request. The client is informing the server that it plans
  to access the area specified by *offset* and *length*. The server
  MAY use this information to speed up further access to that area
  (for example, by performing the actions of `NBD_CMD_READ` but
  replying with just status instead of a payload, by using
  posix_fadvise(), or by retrieving remote data into a local cache
  so that future reads and unaligned writes to that region are
  faster). However, it is unspecified what the server's actual
  caching mechanism is (if any), whether there is a limit on how
  much can be cached at once, and whether writes to a cached region
  have write-through or write-back semantics. Thus, even when this
  command reports success, there is no guarantee of an actual
  performance gain. A future version of this standard may add
  command flags to request particular caching behaviors, where a
  server would reply with an error if that behavior cannot be
  accomplished.

  If an error occurs, the server MUST set the appropriate error code
  in the error field. However failure on this operation does not
  imply that further read and write requests on this area will fail,
  and, other than any difference in performance, there MUST NOT be
  any difference in semantics compared to if the client had not used
  this command. When no command flags are in use, the server MAY
  send a reply prior to the requested area being fully cached.

  Note that client implementations exist which attempt to send a
  cache request even when `NBD_FLAG_SEND_CACHE` was not set in the
  transmission flags field, however, these implementations do not
  use any command flags. A server MAY advertise
  `NBD_FLAG_SEND_CACHE` even if the command has no effect or always
  fails with `NBD_EINVAL`; however, if it advertised the command, the
  server MUST reject any command flags it does not recognize.

- `NBD_CMD_WRITE_ZEROES` (6)

  A write request with no payload. *Offset* and *length* define the
  location and amount of data to be zeroed. The client SHOULD NOT
  request a write length of 0; the behavior of a server on such a
  request is unspecified although the server SHOULD NOT disconnect.

  The server MUST zero out the data on disk, and then send the reply
  message. The server MAY send the reply message before the data has
  reached permanent storage, unless `NBD_CMD_FLAG_FUA` is in use.

  A client MUST NOT send a write zeroes request unless
  `NBD_FLAG_SEND_WRITE_ZEROES` was set in the transmission flags
  field. Additionally, a client MUST NOT send the
  `NBD_CMD_FLAG_FAST_ZERO` flag unless `NBD_FLAG_SEND_FAST_ZERO` was
  set in the transmission flags field.

  By default, the server MAY use trimming to zero out the area, even
  if it did not advertise `NBD_FLAG_SEND_TRIM`; but it MUST ensure
  that the data reads back as zero. However, the client MAY set the
  command flag `NBD_CMD_FLAG_NO_HOLE` to inform the server that the
  area MUST be fully provisioned, ensuring that future writes to the
  same area will not cause fragmentation or cause failure due to
  insufficient space.

  If the server advertised `NBD_FLAG_SEND_FAST_ZERO` but
  `NBD_CMD_FLAG_FAST_ZERO` is not set, then the server MUST NOT fail
  with `NBD_ENOTSUP`, even if the operation is no faster than a
  corresponding `NBD_CMD_WRITE`. Conversely, if
  `NBD_CMD_FLAG_FAST_ZERO` is set, the server MUST fail quickly with
  `NBD_ENOTSUP` unless the request can be serviced in less time than
  a corresponding `NBD_CMD_WRITE`, and SHOULD NOT alter the contents
  of the export when returning this failure. The server's
  determination on whether to fail a fast request MAY depend on a
  number of factors, such as whether the request was suitably
  aligned, on whether the `NBD_CMD_FLAG_NO_HOLE` flag was present,
  or even on whether a previous `NBD_CMD_TRIM` had been performed on
  the region. If the server did not advertise
  `NBD_FLAG_SEND_FAST_ZERO`, then it SHOULD NOT fail with
  `NBD_ENOTSUP`, regardless of the speed of servicing a request, and
  SHOULD fail with `NBD_EINVAL` if the `NBD_CMD_FLAG_FAST_ZERO` flag
  was set. A server MAY advertise `NBD_FLAG_SEND_FAST_ZERO` whether
  or not it will actually succeed on a fast zero request (a fast
  failure of `NBD_ENOTSUP` still counts as a fast response);
  similarly, a server SHOULD fail a fast zero request with
  `NBD_ENOTSUP` if the server cannot quickly determine in advance
  whether proceeding with the request would be fast, even if it
  turns out that the same request without the flag would be fast
  after all.

  One intended use of a fast zero request is optimizing the copying
  of a sparse image source into the export: a client can request
  fast zeroing of the entire export, and if it succeeds, follow that
  with write requests to just the data portions before a single
  flush of the entire image, for fewer transactions overall. On the
  other hand, if the fast zero request fails, the fast failure lets
  the client know that it must manually write zeroes corresponding
  to the holes of the source image before a final flush, for more
  transactions but with no time lost to duplicated I/O to the data
  portions. Knowing this usage pattern can help decide whether a
  server's implementation for writing zeroes counts as fast (for
  example, a successful fast zero request may start a background
  operation that would cause the next flush request to take longer,
  but that is okay as long as intermediate writes before that flush
  do not further lengthen the time spent on the overall sequence of
  operations).

  If an error occurs, the server MUST set the appropriate error code
  in the error field.

  The server SHOULD return `NBD_ENOSPC` if it receives a write zeroes request
  including one or more sectors beyond the size of the device. It SHOULD
  return `NBD_EPERM` if it receives a write zeroes request on a read-only export.

- `NBD_CMD_BLOCK_STATUS` (7)

  A block status query request. Length and offset define the range
  of interest. The client SHOULD NOT request a status length of 0;
  the behavior of a server on such a request is unspecified although
  the server SHOULD NOT disconnect.

  A client MUST NOT send `NBD_CMD_BLOCK_STATUS` unless within the
  negotiation phase it sent `NBD_OPT_SET_META_CONTEXT` at least
  once, and where the final time that was sent, it referred to the
  same export name used to enter transmission phase, and where the
  server returned at least one metadata context without an error.
  This in turn requires the client to first negotiate structured
  replies. For a successful return, the server MUST use a structured
  reply, containing exactly one chunk of type
  `NBD_REPLY_TYPE_BLOCK_STATUS` per selected context id, where the
  status field of each descriptor is determined by the flags field
  as defined by the metadata context. The server MAY send chunks in
  a different order than the context ids were assigned in reply to
  `NBD_OPT_SET_META_CONTEXT`.

  The list of block status descriptors within the
  `NBD_REPLY_TYPE_BLOCK_STATUS` chunk represent consecutive portions
  of the file starting from specified *offset*. If the client used
  the `NBD_CMD_FLAG_REQ_ONE` flag, each chunk contains exactly one
  descriptor where the *length* of the descriptor MUST NOT be greater
  than the *length* of the request; otherwise, a chunk MAY contain
  multiple descriptors, and the final descriptor MAY extend beyond
  the original requested size if the server can determine a larger
  length without additional effort. On the other hand, the server MAY
  return less data than requested. However the server MUST return at
  least one status descriptor (and since each status descriptor has
  a non-zero length, a client can always make progress on a
  successful return). The server SHOULD use different *status*
  values between consecutive descriptors where feasible, although
  the client SHOULD be prepared to handle consecutive descriptors
  with the same *status* value. The server SHOULD use descriptor
  lengths that are an integer multiple of 512 bytes where possible
  (the first and last descriptor of an unaligned query being the
  most obvious places for an exception), and MUST use descriptor
  lengths that are an integer multiple of any advertised minimum
  block size. The status flags are intentionally defined so that a
  server MAY always safely report a status of 0 for any block,
  although the server SHOULD return additional status values when
  they can be easily detected.

  If an error occurs, the server SHOULD set the appropriate error
  code in the error field of an error chunk. However, if the error
  does not involve invalid usage (such as a request beyond the
  bounds of the file), a server MAY reply with a single block status
  descriptor with *length* matching the requested length, rather
  than reporting the error; in this case the context MAY mandate the
  status returned.

  A client MAY initiate a hard disconnect if it detects that the
  server has sent an invalid chunk. The server SHOULD return
  `NBD_EINVAL` if it receives a `NBD_CMD_BLOCK_STATUS` request including
  one or more sectors beyond the size of the device.

- `NBD_CMD_RESIZE` (8)

  Defined by the experimental `RESIZE`
  [extension](https://github.com/NetworkBlockDevice/nbd/blob/extension-resize/doc/proto.md).

- Other requests

  Some third-party implementations may require additional protocol
  messages which are not described in this document. In the interest of
  interoperability, authors of such implementations SHOULD contact the
  maintainer of this document, so that these messages can be listed here
  to avoid conflicting implementations.

#### Error values

The error values are used for the error field in the reply message.
Originally, error messages were defined as the value of `errno` on the
system running the server; however, although they happen to have similar
values on most systems, these values are in fact not well-defined, and
therefore not entirely portable.

Therefore, the allowed values for the error field have been restricted
to set of possibilities. To remain intelligible with older clients, the
most common values of `errno` for that particular error has been chosen
as the value for an error.

The following error values are defined:

- `NBD_EPERM` (1), Operation not permitted.
- `NBD_EIO` (5), Input/output error.
- `NBD_ENOMEM` (12), Cannot allocate memory.
- `NBD_EINVAL` (22), Invalid argument.
- `NBD_ENOSPC` (28), No space left on device.
- `NBD_EOVERFLOW` (75), Value too large.
- `NBD_ENOTSUP` (95), Operation not supported.
- `NBD_ESHUTDOWN` (108), Server is in the process of being shut down.

The server SHOULD return `NBD_ENOSPC` if it receives a write request
including one or more sectors beyond the size of the device. It also
SHOULD map the `EDQUOT` and `EFBIG` errors to `NBD_ENOSPC`. It SHOULD
return `NBD_EINVAL` if it receives a read or trim request including one or
more sectors beyond the size of the device, or if a read or write
request is not aligned to advertised minimum block sizes. Finally, it
SHOULD return `NBD_EPERM` if it receives a write or trim request on a
read-only export.

The server SHOULD NOT return `NBD_EOVERFLOW` except as documented in
response to `NBD_CMD_READ` when `NBD_CMD_FLAG_DF` is supported.

The server SHOULD NOT return `NBD_ENOTSUP` except as documented in
response to `NBD_CMD_WRITE_ZEROES` when `NBD_CMD_FLAG_FAST_ZERO` is
supported.

The server SHOULD return `NBD_EINVAL` if it receives an unknown command.

The server SHOULD return `NBD_EINVAL` if it receives an unknown
command flag. It also SHOULD return `NBD_EINVAL` if it receives a
request with a flag not explicitly documented as applicable to the
given request.

Which error to return in any other case is not specified by the NBD
protocol.

The server SHOULD NOT return `NBD_ENOMEM` if at all possible.

The client SHOULD treat an unexpected error value as if it had been
`NBD_EINVAL`, rather than disconnecting from the server.

## Experimental extensions

In addition to the normative elements of the specification set out
herein, various experimental non-normative extensions have been
proposed. These may not be implemented in any known server or client,
and are subject to change at any point. A full implementation may
require changes to the specifications, or cause the specifications to
be withdrawn altogether.

These experimental extensions are set out in git branches starting
with names starting with the word 'extension'.

Currently known are:

- The `STRUCTURED_REPLY` [extension](https://github.com/NetworkBlockDevice/nbd/blob/extension-structured-reply/doc/proto.md).

- The `BLOCK_STATUS` [extension](https://github.com/NetworkBlockDevice/nbd/blob/extension-blockstatus/doc/proto.md) (based on the `STRUCTURED_REPLY` extension).

- The `RESIZE` [extension](https://github.com/NetworkBlockDevice/nbd/blob/extension-resize/doc/proto.md).

Implementors of these extensions are strongly suggested to contact the
[mailinglist](mailto:nbd@other.debian.org) in order to help
fine-tune the specifications before committing to a particular
implementation.

Those proposing further extensions should also contact the
[mailinglist](mailto:nbd@other.debian.org). It is
possible to reserve command codes etc. within this document
for such proposed extensions. Aside from that, extensions are
written as branches which can be merged into master if and
when those extensions are promoted to the normative version
of the document in the master branch.

## Compatibility and interoperability

Originally, the NBD protocol was a fairly simple protocol with few
options. While the basic protocol is still reasonably simple, a growing
number of extensions has been implemented that may make the protocol
description seem overwhelming at first.

In an effort to not overwhelm first-time implementors with various
options and features that may or may not be important for their use
case, while at the same time desiring maximum interoperability, this
section tries to clarify what is optional and what is expected to be
available in all implementations.

All protocol options and messages not explicitly mentioned below should
be considered optional features that MAY be negotiated between client
and server, but are not required to be available.

### Baseline

The following MUST be implemented by all implementations, and should be
considered a baseline:

- NOTLS mode
- The fixed newstyle handshake
- During the handshake:

  - the `NBD_OPT_INFO` and `NBD_OPT_GO` messages, with the
    `NBD_INFO_EXPORT` response.
  - Servers that receive messages which they do not implement MUST
    reply to them with `NBD_OPT_UNSUP`, and MUST NOT fail to parse
    the next message received.
  - the `NBD_OPT_ABORT` message, and its response.
  - the `NBD_OPT_LIST` message and its response.

- During the transmission phase:

  - Simple replies
  - the `NBD_CMD_READ` message (and its response)
  - the `NBD_CMD_WRITE` message (and its response), unless the
    implementation is a client that does not wish to write
  - the `NBD_CMD_DISC` message (and its resulting effects, although
    no response is involved)

Clients that wish to use more messages MUST negotiate them during the
handshake phase, first.

### Maximum interoperability

Clients and servers that desire maximum interoperability SHOULD
implement the following features:

- TLS-encrypted communication, which may be required by some
  implementations or configurations;
- Servers that implement block constraints through `NBD_INFO_BLOCK_SIZE`
  and desire maximum interoperability SHOULD NOT require them.
  Similarly, clients that desire maximum interoperability SHOULD
  implement querying for block size constraints. Since some clients
  default to a block size of 512 bytes, implementations desiring maximum
  interoperability MAY default to that size.
- Clients or servers that desire interoperability with older
  implementations SHOULD implement the `NBD_OPT_EXPORT_NAME` message in
  addition to `NBD_OPT_INFO` and `NBD_OPT_GO`.
- For data safety, implementing `NBD_CMD_FLUSH` and the
  `NBD_CMD_FLAG_FUA` flag to `NBD_CMD_WRITE` is strongly recommended.
  Clients that do not implement querying for block size constraints
  SHOULD abide by the rules laid out in the section "Block size
  constraints", above.

### Future considerations

The following may be moved to the "Maximum interoperability" or
"Baseline" sections at some point in the future, but some significant
implementations are not yet ready to support them:

- Structured replies; the Linux kernel currently does not yet implement
  them.

## About this file

This file tries to document the NBD protocol as it is currently
implemented in the Linux kernel and in the reference implementation. The
purpose of this file is to allow people to understand the protocol
without having to read the code. However, the description above does not
come with any form of warranty; while every effort has been taken to
avoid them, mistakes are possible.

In contrast to the other files in this repository, this file is not
licensed under the GPLv2. To the extent possible by applicable law, I
hereby waive all copyright and related or neighboring rights to this
file and release it into the public domain.

The purpose of releasing this into the public domain is to allow
competing implementations of the NBD protocol without those
implementations being considered derivative implementations; but please
note that changing this document, while allowed by its public domain
status, does not make an incompatible implementation suddenly speak the
NBD protocol.
