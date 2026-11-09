//! Built-in operations, carried out by the run-time library (`kv_rt`).

mod codes;
mod groups;

/// Built-in methods and standard library functions. Each has up to three
/// operands; the result type says what it gives back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtOp {
    StrLen,
    StrIsEmpty,
    StrUpper,
    StrLower,
    StrTrim,
    StrContains,
    StrStartsWith,
    StrEndsWith,
    StrRepeat,
    ListLen,
    ListIsEmpty,
    ListPush,
    ListGet,
    ListFirst,
    ListLast,
    ListContains,
    WrappingAdd,
    WrappingSub,
    WrappingMul,
    CheckedAdd,
    CheckedSub,
    CheckedMul,
    CheckedDiv,
    /// `list[i] = v`; panics when out of range.
    ListSet,
    MapGet,
    MapInsert,
    MapRemove,
    MapContains,
    MapLen,
    MapIsEmpty,
    MapKeys,
    MapValues,
    /// The entries as a list of `(key, value)` tuples, for `for`.
    MapEntries,
    SharedNew,
    SharedGet,
    SharedSet,
    /// `update` takes and lets go of the value's lock around its function
    /// (decision 0052).
    SharedLock,
    SharedUnlock,
    /// More methods of text and lists (spec 14.2).
    StrReplace,
    StrLines,
    StrChars,
    StrIndexOf,
    StrSlice,
    ListJoin,
    ListSort,
    /// Sorts the list (first operand) by the keys in the second list.
    ListSortBy,
    ListReverse,
    ListTake,
    ListSkip,
    ListIndexOf,
    ListMin,
    ListMax,
    /// `std.fs` (spec 14.3). The ones that can fail leave their error
    /// for `kv_take_error`, like `parse`.
    FsReadText,
    FsWriteText,
    FsAppendText,
    FsExists,
    FsList,
    FsRemove,
    FsCreateDir,
    /// `std.time`, `std.math`, `std.io`, `std.cli` (spec 14.4–14.6).
    TimeNowMs,
    TimeMonotonicMs,
    TimeSleepMs,
    MathSqrt,
    MathPow,
    MathFloor,
    MathCeil,
    MathRound,
    IoReadLine,
    CliArgs,
    CliExit,
    /// `cli.parse<T>()`: the arguments as a tuple of optionals, one per
    /// field (`lower_cli.rs`).
    CliParse,
    /// `std.regex` (spec 14.8). Compiling can fail.
    RegexNew,
    RegexAdvanced,
    RegexIsMatch,
    RegexFind,
    RegexFindAll,
    RegexCaptures,
    RegexNamedCaptures,
    RegexReplace,
    RegexReplaceAll,
    RegexSplit,
    TimeWaitUntil,
    /// `map[key] ?? fallback` for a scalar value and a fallback that is a
    /// constant or a local: the value, or the fallback (nothing boxed).
    MapGetOr,
    StrSplit,
    /// `std.json` (spec 2.12): parsing can fail, like `parse`.
    JsonParse,
    JsonStringify,
    /// `json.of(value)`: the value comes in a one-item tuple, whose type
    /// tells the runtime how to encode it.
    JsonOf,
    /// `json.decode<T>` and `json.decode_value<T>`: they can fail. The
    /// third operand holds the defaults of record fields
    /// (`runtime/native/src/rt_json_decode.rs`).
    JsonDecode,
    JsonDecodeValue,
    /// Tasks (spec 9, `lower_tasks.rs`): `spawn` with the task's closure;
    /// `.wait()`, which gives the task's error when it failed; how many
    /// tasks the running one has spawned, and waiting for those spawned
    /// after such a mark (a function returning, 9.2); and a task's value
    /// or error, handed over when it ends.
    TaskSpawn,
    TaskWait,
    TaskMark,
    TaskJoin,
    TaskDone,
    TaskFail,
    /// Whether the running task is being cancelled (`cancel.rs`).
    TaskCancelling,
    /// `task.cancel()` (spec 9.6): the task stops at once, even while it
    /// only computes.
    TaskCancel,
    /// A point where a task cancelled with `cancel()` stops: a function's
    /// start, a loop round's (`stop_points.rs`). For such a task it sets
    /// the cancelling flag, as a cancelled wait does.
    TaskStopPoint,
    /// `spawn … within d` (spec 9.6): the task (given back) stops once the
    /// milliseconds given have passed.
    TaskWithin,
    /// `spawn process f(args)` (spec 9.8, `lower_process.rs`): in the
    /// parent's task, the call run in a child (it can fail); in the child,
    /// which call it runs, its arguments, and its value or error given
    /// back.
    ProcessRun,
    ProcessSite,
    ProcessArgs,
    ProcessDone,
    ProcessFailed,
    /// A send to an agent (`bot.request(order)`, spec 8.8): the closure of
    /// its thunk, run at once, the agent, and the handler's name. Its value;
    /// it fails with the handler's error, or `panicked: …` when the
    /// handler panics (decision 0076, `lower_events.rs`).
    AgentRun,
    /// `.timeout(d)` (spec 9.6) on `task.wait()`, `receive.take()` and
    /// `send.put(v)`: the same waits, for at most the milliseconds given
    /// last. They can fail.
    TaskWaitFor,
    ChanTakeFor,
    ChanPutFor,
    /// Channels (spec 9.4): a new one (both ends, as a tuple), `put`,
    /// `close`, and `take` (an optional).
    ChanNew,
    ChanPut,
    ChanClose,
    ChanTake,
    /// `for v in receive` over values held by value (spec 9.4): the next
    /// value itself, with no optional made for it; `ChanEnded` then says
    /// whether there was none, as the channel was closed and empty.
    ChanTakeValue,
    ChanEnded,
    /// `Bytes` (spec 2.13): from a list, from text, `b[i]` (which panics
    /// out of range), its methods, `a + b`; and `std.fs` for bytes, which
    /// can fail.
    BytesNew,
    StrBytes,
    BytesAt,
    BytesLen,
    BytesGet,
    BytesSlice,
    BytesText,
    BytesToList,
    BytesAsList,
    BytesConcat,
    FsReadBytes,
    FsWriteBytes,
    FsWriteDurable,
    BytesCrc32c,
    /// Streamed reads (spec 14.3): opening (which can fail), and the next
    /// line or piece, an optional.
    FsLines,
    FsChunks,
    ReaderNext,
    /// `uses Fs("folder")` (spec 8.1): file access kept inside the folder
    /// until the matching leave; enter gives the depth to go back to.
    FsScopeEnter,
    FsScopeLeave,
    /// `std.net` (spec 14.10). All but `address` and `close` can fail, and
    /// all but those and `listen` wait.
    NetListen,
    NetConnect,
    NetAccept,
    NetAddress,
    NetRead,
    NetWrite,
    NetClose,
    /// `net.connect_tls(address)` (decision 0021).
    NetConnectTls,
    /// UDP: `net.bind`, `sock.send(data, to)`, `sock.receive()`. They can
    /// fail; sending and receiving wait.
    NetBind,
    NetSend,
    NetReceive,
    /// Sets the deadline, in ms, of the network op that comes right after
    /// it (`.timeout(d)`, spec 14.10).
    NetDeadline,
    /// `conn.protocol()`: what TLS agreed to speak (ALPN).
    NetProtocol,
    /// `io.is_terminal()`: whether standard output is a terminal (spec 14.6).
    IoIsTerminal,
    /// `io.write(text)`: text without a line end, shown at once (14.6).
    IoWrite,
    /// QUIC (spec 14.10, decision 0022): `net.connect_quic(address,
    /// protocols)`; a connection's `open_stream(two_way)`,
    /// `write_stream(stream, data, done)` (the stream and `done` in one
    /// operand: `stream * 2 + done`) and `read_stream(stream)`. All wait
    /// and can fail.
    NetConnectQuic,
    NetOpenStream,
    NetWriteStream,
    NetReadStream,
    /// Agents (spec 8.3, `lower_agents.rs`): a new guard from the agent's
    /// description; the stack of running agents (push the agent and its
    /// kind, pop, the running one if of this kind).
    AgentStart,
    AgentPush,
    AgentPop,
    AgentCurrent,
    /// The checks of one capability call (`lower_guard.rs`): begin with
    /// the guard, the call's number and its arguments; the totals `count`
    /// and `sum(x)` of a rule; a rule's verdict (0 passes, 1 refuses, 2
    /// waited for the `ask`s under way: check again); an approval's condition
    /// held; who approves; why it was refused; done (denied, ran with a
    /// value, failed with an error).
    AgentBegin,
    AgentCount,
    AgentSum,
    AgentLimit,
    AgentNeed,
    AgentDecide,
    AgentReason,
    AgentDone,
    /// `ask<T>(prompt)` (spec 8.4) with the request the compiler wrote
    /// and the decoding templates of `T`: waits, can fail.
    AiAsk,
    /// `std.log` (spec 14.12): the `[log]` settings, passed once at the
    /// start of `main`; a line at a level.
    LogConfigure,
    LogWrite,
    /// OpenTelemetry export (spec 14.12): the collector, passed once at
    /// the start of `main`; an event's level, line and type name.
    OtlpConfigure,
    OtlpExport,
    /// Durable agents (spec 8.5, `runtime/native/src/rt_journal.rs`):
    /// open an agent's journal; the next unfinished event and its
    /// arguments; an event begins and ends; before a call, what the
    /// journal has for it, and that value; after it, its outcome.
    JournalOpen,
    JournalNext,
    JournalArg,
    JournalBegin,
    JournalEnd,
    JournalRecorded,
    JournalValue,
    JournalRecord,
    /// Raw memory (spec 6.4, `lower_ptr.rs`): `alloc` (count, item size)
    /// and `free`, C's own; a load from and a store to an address the
    /// compiler worked out, which the backends inline; and a text copied
    /// for C with a zero byte after it (10.7), freed after the call; and
    /// C's text at an address, copied (`none` for a null pointer).
    PtrAlloc,
    PtrFree,
    PtrRead,
    PtrWrite,
    PtrText,
    PtrReadText,
    /// A `Float`'s IEEE 754 bits and back (spec 14.2): the slot already
    /// holds them, so the runtime gives its operand back.
    FloatBits,
    FloatFromBits,
    /// A chunked loop on all cores (decision 0031): the backends call
    /// `kv_par_for`, or give 1 ("run the plain loop yourself").
    ParFor,
    /// A chunked integer reduction (decision 0031): `kv_par_for_red`,
    /// whose chunks also get a slot number for their partial sums.
    ParForRed,
    /// `net.listen_tls` (spec 14.10): the address, the protocols, and the
    /// certificate, key and clients' roots in a list (`lower_tls.rs`).
    NetListenTls,
    /// `panic(message)` with a message computed while the program runs
    /// (spec 5.6); a written-out message is `ExprKind::Panic`.
    PanicText,
    /// `Map.with_capacity(n)` and `Set.with_capacity(n)` (spec 2.2): an
    /// empty table with room for `n` entries.
    MapWithCapacity,
    /// `random_bits()` (spec 14.15): 64 random bits for `std.random`.
    RandomBits,
    /// `b.xxhash64()` (spec 2.13).
    BytesXxhash64,
    /// `io.colors()`: whether standard output shows colors (spec 14.6).
    IoColors,
    /// The next item of a reader whose loop header has `try` or `catch`
    /// (spec 14.3): a failed read sets an `FsError` instead of panicking.
    ReaderTryNext,
    /// A hint that item `b` of list `a` will be read soon (`prefetch.rs`):
    /// nothing a program sees; a backend may ignore it.
    ListPrefetch,
    /// A hint that integer key `b` of map `a` will be looked up soon
    /// (`prefetch.rs`): its bucket is asked for.
    MapPrefetch,
    /// `io.read_hidden()` (spec 14.6): a line typed unseen.
    IoReadHidden,
    /// `a ** b` for whole numbers of the result's type (spec 1.8): panics
    /// when `b` is negative or the result doesn't fit.
    IntPow,
    /// `a.near(b)` on floats (spec 14.2).
    FloatNear,
    /// `list.map(f)` on the GPU (spec 15.2): the list, the closure and the
    /// kernel's WGSL; gives the new list, or `none` for the CPU to do it.
    GpuMap,
    /// With `KAVACH_GPU=check`: the CPU's list from a `map` the GPU also
    /// ran, compared with the GPU's; gives the CPU's list.
    GpuCheck,
    /// `GpuMap` whose result may stay in GPU memory only: every read of
    /// it is a `GpuFetch` first (`gpu_stay.rs`).
    GpuMapStay,
    /// A list, with its items brought back from GPU memory if a
    /// `GpuMapStay` left them only there.
    GpuFetch,
    /// `list.sum(f)` of `Int32` or `UInt32` on the GPU (spec 15.2): the
    /// list, the closure and the kernel's WGSL; gives the sum when
    /// `GpuReduced` says the GPU did it.
    GpuSum,
    /// `list.min()` on the GPU: the list, a placeholder and the kernel.
    GpuMin,
    /// `list.max()` on the GPU, as `GpuMin`.
    GpuMax,
    /// Whether the last `GpuSum`, `GpuMin` or `GpuMax` gave its value; if
    /// not, the CPU works it out, in order.
    GpuReduced,
    /// A `GpuSum`, `GpuMin` or `GpuMax` that doesn't wait (`gpu_later.rs`):
    /// its value comes through the `GpuLaterNext` after it.
    GpuSumLater,
    GpuMinLater,
    GpuMaxLater,
    /// A new queue of reductions that didn't wait.
    GpuLaterNew,
    /// Adds the reduction just made to queue `a`, and gives whether one is
    /// due, which its statement then runs for: the oldest, when too many
    /// are under way or the CPU must do the newest, or when `b` is 1 and
    /// any is left.
    GpuLaterNext,
    /// The value of the reduction `GpuLaterNext` gave; `GpuReduced` says
    /// whether it has one.
    GpuLaterValue,
    /// Its list, for the CPU to work it out when it has no value.
    GpuLaterList,
    /// `list.filter(f)` on the GPU (spec 15.2): the list, the closure and
    /// the kernel's WGSL; gives the new list, or `none` for the CPU to do
    /// it.
    GpuFilter,
    /// `GpuFilter` whose result may stay in GPU memory only, as
    /// `GpuMapStay`.
    GpuFilterStay,
    /// `tokens` in a `limit Ai` rule (spec 8.3): the call being checked
    /// and the rule; gives what the window's asks took, with those under
    /// way counted as the agent's settings say.
    AgentTokens,
    /// At the start of a program with GPU kernels: open the GPU and
    /// compile the kernels (its text, joined by `\0`) on a background
    /// thread, while the program does its first work.
    GpuWarm,
    /// At the start of a program with a `spawn cores` (spec 9.8): a
    /// worker thread per core, for those tasks.
    TaskParallel,
    /// At the start of a program with `Drop` (spec 6.1): type number `a`
    /// runs closure `b` (its `drop`) when its last holder ends.
    DropRegister,
    /// At a statement boundary: run the `drop` of the values whose last
    /// holder has ended (`runtime/native/src/rt_drops.rs`).
    DropRun,
    /// `d.round(places)` (`c` is 0) and `d.round_even(places)` (`c` is 1)
    /// on a `Decimal` (spec 14.2).
    DecRound,
    /// `match` on an `Error` (spec 5.5): the error it holds, past any
    /// `context`, as the optional of the result's type: `none` when it is
    /// of another type.
    ErrorAs,
    /// `time.zone_offset(name, ms)` (spec 14.4): seconds ahead of UTC, or
    /// `none` for an unknown zone (`runtime/native/src/rt_zone.rs`).
    ZoneOffset,
    /// `agents.call_key()` (spec 14.16): the running call's idempotency
    /// key, or `none` (`runtime/native/src/rt_journal.rs`).
    JournalCallKey,
    /// `clipboard.read_text()` and `write_text(text)` (spec 14.17);
    /// both can fail (`runtime/native/src/rt_clipboard.rs`).
    ClipReadText,
    ClipWriteText,
    /// Serial ports (spec 14.18), by number: the ports there are, open
    /// (can fail), read and write (can fail), close
    /// (`runtime/native/src/rt_serial.rs`).
    SerialPorts,
    SerialOpen,
    SerialRead,
    SerialWrite,
    SerialClose,
    /// `s.weak()` and `w.shared()` (spec 6.2): a `Weak` has the shape of
    /// its `Shared`, around a weak cell (`runtime/native/src/send.rs`).
    WeakMake,
    WeakShared,
    /// `approve … within d`: the request may wait `d` (decision 0072).
    AgentWithin,
    /// `agents.request_fields()` and `agents.answer_request(fields)` (spec
    /// 14.16), with the project's agents folder first; answering can fail
    /// (`runtime/native/src/rt_approvals.rs`).
    AgentApprovals,
    AgentAnswer,
    /// An event run again after a restart (decision 0072,
    /// `runtime/native/src/rt_replays.rs`): its task, from closure `a`,
    /// of agent `b`; `start` waits for it until it ends or asks a person.
    AgentReplay,
    AgentReplayWait,
    /// The replayed event ended: its value (`b` is 1) or error (2), in
    /// tuple `c`, to the audit log when `start` no longer waits for it.
    AgentReplayEnd,
    /// Whether a replayed event of handler `b` with arguments in tuple `c`
    /// still waits for a person; then its task, for the send to join.
    AgentJoining,
    AgentJoined,
    /// An `ask`'s check (spec 8.3): `b` is its model's `max_tokens`; with
    /// the prompt's length, the most the `ask` can take.
    AgentBound,
    /// At the start of `main` (spec 10.5): what built the program, between
    /// its marks; whether it answers `--version` with it; and the
    /// compiler's `RtOp::FINGERPRINT`, which the runtime checks.
    CliVersion,
}
