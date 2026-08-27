#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CommandMetadata {
    pub(crate) name: &'static str,
    pub(crate) arity: i64,
    pub(crate) flags: &'static [&'static str],
    pub(crate) first_key: i64,
    pub(crate) last_key: i64,
    pub(crate) key_step: i64,
}

const READ: &[&str] = &["readonly", "fast"];
const WRITE: &[&str] = &["write"];
const CONNECTION: &[&str] = &["noscript", "loading", "stale"];
const ADMIN: &[&str] = &["admin"];
const NONE: &[&str] = &[];

macro_rules! metadata {
    ($name:literal, $arity:expr, $flags:expr, $first:expr, $last:expr, $step:expr) => {
        CommandMetadata {
            name: $name,
            arity: $arity,
            flags: $flags,
            first_key: $first,
            last_key: $last,
            key_step: $step,
        }
    };
}

pub(crate) const COMMANDS: &[CommandMetadata] = &[
    metadata!("aofrewrite", 1, ADMIN, 0, 0, 0),
    metadata!("append", 3, WRITE, 1, 1, 1),
    metadata!("clear", 1, WRITE, 0, 0, 0),
    metadata!("client", -2, CONNECTION, 0, 0, 0),
    metadata!("command", -1, CONNECTION, 0, 0, 0),
    metadata!("dbsize", 1, READ, 0, 0, 0),
    metadata!("decr", 2, WRITE, 1, 1, 1),
    metadata!("decrby", 3, WRITE, 1, 1, 1),
    metadata!("del", -2, WRITE, 1, -1, 1),
    metadata!("echo", 2, CONNECTION, 0, 0, 0),
    metadata!("exists", -2, READ, 1, -1, 1),
    metadata!("exit", 1, CONNECTION, 0, 0, 0),
    metadata!("expire", 3, WRITE, 1, 1, 1),
    metadata!("flushall", -1, WRITE, 0, 0, 0),
    metadata!("flushdb", -1, WRITE, 0, 0, 0),
    metadata!("get", 2, READ, 1, 1, 1),
    metadata!("getdel", 2, WRITE, 1, 1, 1),
    metadata!("getex", -2, WRITE, 1, 1, 1),
    metadata!("getrange", 4, READ, 1, 1, 1),
    metadata!("getset", 3, WRITE, 1, 1, 1),
    metadata!("hello", -1, CONNECTION, 0, 0, 0),
    metadata!("help", 1, NONE, 0, 0, 0),
    metadata!("incr", 2, WRITE, 1, 1, 1),
    metadata!("incrby", 3, WRITE, 1, 1, 1),
    metadata!("incrbyfloat", 3, WRITE, 1, 1, 1),
    metadata!("info", 1, READ, 0, 0, 0),
    metadata!("keys", 1, READ, 0, 0, 0),
    metadata!("len", 1, READ, 0, 0, 0),
    metadata!("llen", 2, READ, 1, 1, 1),
    metadata!("lpop", 2, WRITE, 1, 1, 1),
    metadata!("lpush", 3, WRITE, 1, 1, 1),
    metadata!("lrange", 4, READ, 1, 1, 1),
    metadata!("mget", -2, READ, 1, -1, 1),
    metadata!("mset", -3, WRITE, 1, -1, 2),
    metadata!("msetnx", -3, WRITE, 1, -1, 2),
    metadata!("persist", 2, WRITE, 1, 1, 1),
    metadata!("pexpire", 3, WRITE, 1, 1, 1),
    metadata!("ping", -1, CONNECTION, 0, 0, 0),
    metadata!("pttl", 2, READ, 1, 1, 1),
    metadata!("quit", 1, CONNECTION, 0, 0, 0),
    metadata!("rename", 3, WRITE, 1, 2, 1),
    metadata!("rpop", 2, WRITE, 1, 1, 1),
    metadata!("rpush", 3, WRITE, 1, 1, 1),
    metadata!("sadd", 3, WRITE, 1, 1, 1),
    metadata!("save", 1, ADMIN, 0, 0, 0),
    metadata!("scard", 2, READ, 1, 1, 1),
    metadata!("select", 2, CONNECTION, 0, 0, 0),
    metadata!("set", -3, WRITE, 1, 1, 1),
    metadata!("setnx", 3, WRITE, 1, 1, 1),
    metadata!("setrange", 4, WRITE, 1, 1, 1),
    metadata!("sismember", 3, READ, 1, 1, 1),
    metadata!("smembers", 2, READ, 1, 1, 1),
    metadata!("srem", 3, WRITE, 1, 1, 1),
    metadata!("strlen", 2, READ, 1, 1, 1),
    metadata!("ttl", 2, READ, 1, 1, 1),
];

pub(crate) fn command_metadata(name: &[u8]) -> Option<CommandMetadata> {
    COMMANDS
        .iter()
        .copied()
        .find(|metadata| name.eq_ignore_ascii_case(metadata.name.as_bytes()))
}
