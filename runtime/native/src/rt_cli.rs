//! `cli.parse<T>()` in native code (spec 14.6): reads the arguments and
//! converts each to its field's type. Messages must match the
//! interpreter (`compiler/interp/src/cli.rs`).

use crate::heap::{Data, alloc, alloc_slots, new_str, release, shape, str_of};
use crate::rt::{range, type_of};
use crate::rt_errors::set_error;
use kavach_kir::{Shape, Type};
use kavach_runtime_core::Decimal;

/// One field: its option, whether it is required, whether it is a flag,
/// and its type as written (for messages).
struct Spec {
    option: String,
    required: bool,
    flag: bool,
    written: String,
}

fn spec(line: &str) -> Spec {
    let parts: Vec<&str> = line.splitn(4, '|').collect();
    Spec {
        option: parts[0].to_string(),
        required: parts.get(1) == Some(&"1"),
        flag: parts.get(2) == Some(&"flag"),
        written: parts.get(3).unwrap_or(&"").to_string(),
    }
}

/// The groups of options: one unnamed group for a record, or one per
/// command of a union, where each starts with a line `@command`.
fn groups(text: &str) -> Vec<(String, Vec<Spec>)> {
    let mut groups: Vec<(String, Vec<Spec>)> = Vec::new();
    for line in text.lines() {
        if let Some(command) = line.strip_prefix('@') {
            groups.push((command.to_string(), Vec::new()));
            continue;
        }
        if groups.is_empty() {
            groups.push((String::new(), Vec::new()));
        }
        if let Some((_, specs)) = groups.last_mut() {
            specs.push(spec(line));
        }
    }
    groups
}

/// The group the first argument names, or the problem.
fn read_command(args: &[String], groups: &[(String, Vec<Spec>)]) -> Result<usize, String> {
    match args.first().map(String::as_str) {
        None => Err("missing command".into()),
        Some("--help" | "-h") => Err("help:".into()),
        Some(word) => groups
            .iter()
            .position(|(c, _)| c == word)
            .ok_or(format!("unknown command `{word}`")),
    }
}

/// The text given for each field, or the problem.
fn read(args: &[String], specs: &[Spec]) -> Result<Vec<Option<String>>, String> {
    let mut given = vec![None; specs.len()];
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--help" || arg == "-h" {
            return Err("help:".into());
        }
        let Some(word) = arg.strip_prefix("--") else {
            return Err(format!(
                "unexpected argument `{arg}`: options start with `--`"
            ));
        };
        let (name, inline) = match word.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (word, None),
        };
        let Some(i) = specs.iter().position(|s| s.option == name) else {
            return Err(format!("unknown option `--{name}`"));
        };
        given[i] = Some(match (inline, specs[i].flag) {
            (Some(v), _) => v,
            (None, true) => "true".into(),
            (None, false) => rest
                .next()
                .cloned()
                .ok_or(format!("--{name} needs a value"))?,
        });
    }
    Ok(given)
}

/// `text` as a slot of type `t`, owned, or `None` if it isn't one.
fn convert(text: &str, t: Type) -> Option<i64> {
    Some(match t {
        Type::F64 => text.parse::<f64>().ok()?.to_bits() as i64,
        Type::F32 => crate::display::f32_slot(text.parse::<f32>().ok()?),
        Type::Bool => text.parse::<bool>().ok()? as i64,
        Type::Ref(_) => match shape(t) {
            Shape::Decimal => alloc(t, Data::Decimal(Box::new(Decimal::parse(text)?))),
            _ => new_str(text.to_string(), t),
        },
        t => {
            let n: i128 = text.parse().ok()?;
            let (lo, hi) = range(t);
            if n < lo || n > hi {
                return None;
            }
            n as i64
        }
    })
}

/// Fills a tuple (type `code`) of optionals, one per field; for a union
/// of commands, the tuple starts with the index of the command, and only
/// that command's optionals are filled. On a problem, leaves the error
/// (the problem, then the usage) and returns 0.
pub fn cli_parse(spec: i64, usage: i64, code: i64, args: &[String]) -> i64 {
    let (spec_text, usage_text) = (str_of(spec), str_of(usage));
    let str_ty = obj_type(spec);
    release(spec);
    release(usage);
    let groups = groups(&spec_text);
    // The whole usage, then (for a union) each command's.
    let usages: Vec<&str> = usage_text.split('\0').collect();
    let fail = |message: String, usage: &str| {
        let text = if message == "help:" {
            usage.to_string()
        } else {
            format!(
                "{message}
{usage}"
            )
        };
        set_error(new_str(text, str_ty));
        0
    };
    let commands = groups.first().is_some_and(|(c, _)| !c.is_empty());
    let (index, args) = if commands {
        match read_command(args, &groups) {
            Ok(i) => (i, &args[1..]),
            Err(m) => return fail(m, usages[0]),
        }
    } else {
        (0, args)
    };
    let usage = usages.get(index + commands as usize).copied().unwrap_or("");
    let specs = &groups[index].1;
    let given = match read(args, specs) {
        Ok(g) => g,
        Err(m) => return fail(m, usage),
    };
    let slot_types = match shape(type_of(code)) {
        Shape::Tuple(ts) => ts.clone(),
        _ => Vec::new(),
    };
    // Where the command's optionals start in the tuple.
    let first = commands as usize + groups[..index].iter().map(|(_, s)| s.len()).sum::<usize>();
    let mut filled = Vec::new();
    for ((s, g), opt) in specs.iter().zip(given).zip(&slot_types[first..]) {
        let inner = match shape(*opt) {
            Shape::Optional(t) => *t,
            _ => *opt,
        };
        let slot = match g {
            Some(text) => match convert(&text, inner) {
                Some(v) if inner.is_ref() => v,
                Some(v) => alloc(*opt, Data::Boxed(v as u64)),
                None => {
                    filled.iter().for_each(|p| release(*p as i64));
                    let problem = format!("--{} needs {}, not `{text}`", s.option, s.written);
                    return fail(problem, usage);
                }
            },
            None if s.required => {
                filled.iter().for_each(|p| release(*p as i64));
                return fail(format!("missing --{}", s.option), usage);
            }
            None => 0,
        };
        filled.push(slot as u64);
    }
    let mut slots = vec![0; slot_types.len()];
    if commands {
        slots[0] = index as u64;
    }
    slots[first..first + filled.len()].copy_from_slice(&filled);
    alloc_slots(type_of(code), 0, slots)
}

fn obj_type(p: i64) -> Type {
    crate::heap::obj(p).ty()
}
