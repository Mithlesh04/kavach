//! What each op is like: whether it changes its receiver or can fail,
//! and which runtime function carries it out.

use super::RtOp;

impl RtOp {
    /// Ops that change their first operand, a list, map or set held in a
    /// variable, and give back the changed value.
    pub fn changes_receiver(self) -> bool {
        matches!(
            self,
            RtOp::ListPush | RtOp::ListSet | RtOp::MapInsert | RtOp::MapRemove
        )
    }

    /// Ops that can fail (spec 14.3): only inside `LetResult`.
    pub fn can_fail(self) -> bool {
        matches!(
            self,
            RtOp::FsReadText
                | RtOp::EnvGet
                | RtOp::FileInfoStat
                | RtOp::FileInfoSetReadOnly
                | RtOp::FileInfoSetPermissions
                | RtOp::FileInfoRename
                | RtOp::FileInfoCopy
                | RtOp::FileInfoRemoveAll
                | RtOp::CryptoFails
                | RtOp::Base64Decode
                | RtOp::ClipReadText
                | RtOp::ClipWriteText
                | RtOp::SerialOpen
                | RtOp::AgentAnswer
                | RtOp::SerialRead
                | RtOp::SerialWrite
                | RtOp::FsWriteText
                | RtOp::FsAppendText
                | RtOp::FsList
                | RtOp::FsRemove
                | RtOp::FsCreateDir
                | RtOp::CliParse
                | RtOp::RegexNew
                | RtOp::RegexAdvanced
                | RtOp::JsonParse
                | RtOp::JsonDecode
                | RtOp::JsonDecodeValue
                | RtOp::TaskWait
                | RtOp::TaskWaitFor
                | RtOp::FsReadBytes
                | RtOp::FsWriteBytes
                | RtOp::FsWriteDurable
                | RtOp::FsLines
                | RtOp::FsChunks
                | RtOp::ReaderTryNext
                | RtOp::NetListen
                | RtOp::NetConnect
                | RtOp::NetConnectTls
                | RtOp::NetStartTls
                | RtOp::NetAcceptTls
                | RtOp::NetListenTls
                | RtOp::NetBind
                | RtOp::NetSend
                | RtOp::NetReceive
                | RtOp::NetAccept
                | RtOp::NetRead
                | RtOp::NetWrite
                | RtOp::ChanTakeFor
                | RtOp::ChanPutFor
                | RtOp::AiAsk
                | RtOp::NetConnectQuic
                | RtOp::NetOpenStream
                | RtOp::NetWriteStream
                | RtOp::NetReadStream
                | RtOp::ProcessRun
                | RtOp::AgentRun
        )
    }

    /// The runtime function that carries out the op. Each feature group
    /// has its own, so a program that never uses one (regular
    /// expressions, JSON, the network, …) doesn't link its code
    /// (decision 0024).
    pub fn entry(self) -> &'static str {
        let name = format!("{self:?}");
        let group = |prefix: &str| name.starts_with(prefix);
        if group("Regex") {
            "kv_rt_regex"
        } else if group("Json") {
            "kv_rt_json"
        } else if group("Net") {
            "kv_rt_net"
        } else if group("Task") || group("Chan") {
            "kv_rt_tasks"
        } else if group("Agent") {
            "kv_rt_agents"
        } else if group("Otlp") {
            "kv_rt_otlp"
        } else if group("Gpu") {
            "kv_rt_gpu"
        } else if group("Drop") {
            "kv_rt_drops"
        } else if group("Random") {
            "kv_rt_random"
        } else if group("Log") {
            "kv_rt_log"
        } else if group("Journal") {
            "kv_rt_journal"
        } else if group("Zone") {
            "kv_rt_zone"
        } else if group("Crypto") {
            "kv_rt_crypto"
        } else if group("Base64") {
            "kv_rt_base64"
        } else if group("Clip") {
            "kv_rt_clipboard"
        } else if group("FileInfo") {
            "kv_rt_file_info"
        } else if group("Serial") {
            "kv_rt_serial"
        } else if group("Weak") {
            "kv_rt_weak"
        } else if group("Process") {
            "kv_rt_process"
        } else if self == RtOp::CliVersion {
            "kv_rt_version"
        } else if self == RtOp::AiAsk {
            "kv_rt_ai"
        } else {
            "kv_rt"
        }
    }
}
