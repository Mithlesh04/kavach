//! `kavach context` (spec 13.4): the full code of one declaration, plus
//! the outline of everything it uses, cut to a token budget.

use crate::index::{Decl, Index};

/// A rough token count: about four characters per token.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// The context needed to change `name`: a function, a type, a method
/// (`Type.method`) or an agent handler (`Agent.handler`), with or without
/// its module in front (spec 13.9).
pub fn context(index: &Index, name: &str, budget: usize) -> Result<String, String> {
    let target = find_target(index, name)?;
    let mut out = target.code.clone();
    let mut used = estimate_tokens(&out);
    let mut shown = Vec::new();
    let mut skipped = Vec::new();
    for item in target.refers.iter().map(|i| &index.decls[*i]) {
        let cost = estimate_tokens(&item.outline);
        if used + cost > budget {
            skipped.push(item.name.clone());
            continue;
        }
        used += cost;
        shown.push(item.outline.as_str());
    }
    if !shown.is_empty() {
        out.push_str(&format!("\n\n// used by `{name}`:\n\n"));
        out.push_str(&shown.join("\n\n"));
    }
    if !skipped.is_empty() {
        out.push_str(&format!(
            "\n\n// left out to fit {budget} tokens: {}",
            skipped.join(", ")
        ));
    }
    Ok(format!(
        "// context for `{name}` in {} (~{used} tokens)\n{out}\n",
        index.entry
    ))
}

/// The one declaration with code that `name` names. A plain name that
/// several modules declare must be written with its module.
fn find_target<'a>(index: &'a Index, name: &str) -> Result<&'a Decl, String> {
    let found: Vec<&Decl> = index
        .decls
        .iter()
        .filter(|d| !d.code.is_empty() && d.is_named(name))
        .collect();
    match found.as_slice() {
        [] => Err(format!("nothing named `{name}` in {}", index.entry)),
        [one] => Ok(one),
        many => {
            let names: Vec<String> = many
                .iter()
                .map(|d| format!("{}.{}", d.module, d.short_name()))
                .collect();
            Err(format!(
                "several declarations are named `{name}`: write {}",
                names.join(" or ")
            ))
        }
    }
}
