//! What a library exports, and its C header (spec 10.10, decision 0026):
//! every `pub fn` of the library's own file that C can call, under the
//! name `<package>_<fn>`; `<package>_free` for the text it gives; and
//! `<package>_last_error` with a number for each error variant, for
//! functions that can fail.

use kavach_ast::{FnDecl, ItemKind, TypeBody, TypeExpr, TypeKind};
use kavach_driver::Compiled;
use kavach_resolver::DefKind;

/// A library's exported functions, the `pub` functions it doesn't
/// export with the reason, and its C header.
pub struct Library {
    /// Kavach name and C name of each export.
    pub exports: Vec<(String, String)>,
    pub skipped: Vec<(String, String)>,
    pub header: String,
    /// The C name of the function that frees text the library gave.
    pub free: String,
    /// The C name of the function that gives the last error as text.
    pub last_error: String,
}

/// `AgeError` as `AGE_ERROR`, for the names of C constants.
fn upper_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// A `#define` for each variant of error type `t`, numbered from 1, as a
/// failing export returns them; one for an error that isn't a union.
fn error_codes(c: &Compiled, t: &TypeExpr, prefix: &str) -> Vec<String> {
    let TypeKind::Named { path, .. } = &t.kind else {
        return Vec::new();
    };
    let first = &path.segments[0];
    let name = format!("{}_{}", prefix.to_uppercase(), upper_snake(&first.name));
    let declared = c
        .resolution
        .uses
        .get(&first.span.start)
        .and_then(|d| c.resolution.defs[*d].span);
    let variants = c.file.items.iter().find_map(|i| match &i.kind {
        ItemKind::Type(t) if Some(t.name.span) == declared => match &t.body {
            TypeBody::Union(vs) => Some(vs),
            TypeBody::Record(_) | TypeBody::Alias(_) => None,
        },
        _ => None,
    });
    match variants {
        Some(vs) => vs
            .iter()
            .enumerate()
            .map(|(i, v)| format!("#define {name}_{} {}", upper_snake(&v.name.name), i + 1))
            .collect(),
        None => vec![format!("#define {name} 1")],
    }
}

/// The C type of a written Kavach type: text is a `const char*` coming
/// in (`out` false) and a `char*` the caller frees going out.
fn c_type(c: &Compiled, t: &TypeExpr, out: bool) -> Option<String> {
    let TypeKind::Named { path, args } = &t.kind else {
        return None;
    };
    let first = &path.segments[0];
    let def = c.resolution.uses.get(&first.span.start)?;
    let d = &c.resolution.defs[*def];
    if d.kind == DefKind::Type {
        return None;
    }
    Some(match (first.name.as_str(), args.as_slice()) {
        ("Int", []) => "int64_t".into(),
        ("Int32", []) => "int32_t".into(),
        ("Int16", []) => "int16_t".into(),
        ("Int8", []) => "int8_t".into(),
        ("UInt64", []) => "uint64_t".into(),
        ("UInt32", []) => "uint32_t".into(),
        ("UInt16", []) => "uint16_t".into(),
        ("UInt8", []) => "uint8_t".into(),
        ("Float", []) => "double".into(),
        ("Bool", []) => "bool".into(),
        ("String", []) if out => "char*".into(),
        ("String", []) => "const char*".into(),
        ("Ptr", [item]) => match c_type(c, item, false).filter(|t| !t.contains("char*")) {
            Some(inner) => format!("{inner}*"),
            // A pointer to a record (spec 6.4): its layout is the
            // library's business.
            None => "void*".into(),
        },
        _ => return None,
    })
}

/// Why `f` can't be exported, or its C prototype under `c_name`. One that
/// can fail returns its error's number, and its value through `result`.
fn prototype(c: &Compiled, f: &FnDecl, c_name: &str) -> Result<String, String> {
    if !f.generics.is_empty() {
        return Err("it has type parameters".into());
    }
    let mut params = Vec::new();
    for p in &f.params {
        let ty = p.ty.as_ref().ok_or("it takes `self`")?;
        let shown = c.src[ty.span.start as usize..ty.span.end as usize].to_string();
        let cty =
            c_type(c, ty, false).ok_or(format!("it takes `{shown}`, which C can't pass yet"))?;
        params.push(format!("{cty} {}", p.name.name));
    }
    let ret = match &f.ret {
        None => "void".to_string(),
        Some(t) => {
            let shown = &c.src[t.span.start as usize..t.span.end as usize];
            c_type(c, t, true).ok_or(format!("it gives `{shown}`, which C can't take yet"))?
        }
    };
    let ret = match (&f.errors, ret.as_str()) {
        (None, _) => ret,
        (Some(_), "void") => "int32_t".to_string(),
        (Some(_), value) => {
            params.push(format!("{value}* result"));
            "int32_t".to_string()
        }
    };
    let params = if params.is_empty() {
        "void".to_string()
    } else {
        params.join(", ")
    };
    Ok(format!("{ret} {c_name}({params});"))
}

/// The exports of the library `package` compiled in `c`, from its own
/// file (`src/lib.kav`).
pub fn library(c: &Compiled, package: &str) -> Library {
    let prefix = package.replace('-', "_");
    let (mut exports, mut skipped, mut prototypes) = (Vec::new(), Vec::new(), Vec::new());
    let mut codes: Vec<String> = Vec::new();
    for item in c.own_items() {
        let ItemKind::Fn(f) = &item.kind else {
            continue;
        };
        let in_lib = c.location(item.span.start).0 == c.path;
        if !item.is_pub || !in_lib || f.c_library.is_some() {
            continue;
        }
        let c_name = format!("{prefix}_{}", f.name.name);
        match prototype(c, f, &c_name) {
            Ok(p) => {
                prototypes.push(p);
                exports.push((f.name.name.clone(), c_name));
                for code in f.errors.iter().flat_map(|e| error_codes(c, e, &prefix)) {
                    if !codes.contains(&code) {
                        codes.push(code);
                    }
                }
            }
            Err(why) => skipped.push((f.name.name.clone(), why)),
        }
    }
    prototypes.push(format!(
        "\n/* Frees text this library gave (a `char*` result). */\nvoid {prefix}_free(char* text);"
    ));
    prototypes.push(format!(
        "/* The last error, as text to free with {prefix}_free; null before any. */\nchar* {prefix}_last_error(void);"
    ));
    if !codes.is_empty() {
        codes.push(String::new());
        prototypes.insert(0, codes.join("\n"));
    }
    let guard = format!("{}_H", prefix.to_uppercase());
    let header = format!(
        "/* {package}.h: the C interface of the Kavach library `{package}`,\n   made by `kavach build --lib` (spec 10.10). */\n\n\
         #ifndef {guard}\n#define {guard}\n\n#include <stdbool.h>\n#include <stdint.h>\n\n\
         #ifdef __cplusplus\nextern \"C\" {{\n#endif\n\n{}\n\n#ifdef __cplusplus\n}}\n#endif\n\n#endif\n",
        prototypes.join("\n")
    );
    Library {
        exports,
        skipped,
        header,
        free: format!("{prefix}_free"),
        last_error: format!("{prefix}_last_error"),
    }
}
