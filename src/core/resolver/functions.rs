use crate::core::types::*;
use std::collections::HashMap;

/// A builtin template function's implementation.
///
/// It is invoked only after [`check_arg_count`] has confirmed the declared
/// arity, so indexing `args[0..arity]` inside one cannot panic.
type BuiltinImpl = fn(&[String]) -> Result<String, String>;

/// Dispatch table for the builtin template functions: `(name, arity, impl)`.
///
/// Exists so [`resolve_function`] is a lookup-check-apply pipeline rather than a
/// nine-arm match that spelled each function's name twice — once as the match
/// arm, once as the `check_arg_count` string literal — where the two could drift
/// apart silently. Arity now lives beside the name it belongs to.
const BUILTINS: &[(&str, usize, BuiltinImpl)] = &[
    ("upper", 1, |args| Ok(args[0].to_uppercase())),
    ("lower", 1, |args| Ok(args[0].to_lowercase())),
    ("trim", 1, |args| Ok(args[0].trim().to_string())),
    ("default", 2, |args| {
        Ok(if args[0].is_empty() {
            args[1].clone()
        } else {
            args[0].clone()
        })
    }),
    ("replace", 3, |args| {
        Ok(args[0].replace(args[1].as_str(), args[2].as_str()))
    }),
    ("env", 1, |args| {
        std::env::var(&args[0]).map_err(|_| format!("env var '{}' not set", args[0]))
    }),
    ("b3sum", 1, |args| {
        Ok(blake3::hash(args[0].as_bytes()).to_hex().to_string())
    }),
    ("join", 2, |args| {
        // First arg is a comma-separated list, second is the new separator
        let parts: Vec<&str> = args[0].split(',').map(|s| s.trim()).collect();
        Ok(parts.join(&args[1]))
    }),
    ("split", 2, |args| {
        // Split string by delimiter, return comma-separated
        let parts: Vec<&str> = args[0].split(args[1].as_str()).collect();
        Ok(parts.join(","))
    }),
];

/// Resolve a template function call like `upper(params.name)`.
pub(crate) fn resolve_function(
    expr: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<String, String> {
    let (func_name, args_str) = split_call(expr)?;
    let args = parse_func_args(args_str, params, machines)?;

    let (_, arity, apply) = BUILTINS
        .iter()
        .find(|(name, _, _)| *name == func_name)
        .ok_or_else(|| format!("unknown template function: {func_name}"))?;
    check_arg_count(func_name, &args, *arity)?;
    apply(&args)
}

/// Split `name(raw args)` into the trimmed function name and the raw argument
/// text between the parentheses.
///
/// Decides only whether the call is syntactically well-formed — an opening
/// paren must exist and the expression must end with the closing one. Exists so
/// [`resolve_function`] carries no parsing of its own and reads as pure dispatch.
fn split_call(expr: &str) -> Result<(&str, &str), String> {
    let open_paren = expr
        .find('(')
        .ok_or_else(|| format!("malformed function: {expr}"))?;
    if !expr.ends_with(')') {
        return Err(format!("unclosed parenthesis in function: {expr}"));
    }
    Ok((
        expr[..open_paren].trim(),
        &expr[open_paren + 1..expr.len() - 1],
    ))
}

/// Parse function arguments, resolving param/machine references and quoted literals.
fn parse_func_args(
    args_str: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Vec<String>, String> {
    split_top_level_args(args_str)
        .into_iter()
        .map(|arg| resolve_func_arg(&arg, params, machines))
        .collect()
}

/// Split raw argument text on top-level commas, honouring quotes and nested
/// parentheses, and return each argument trimmed (a trailing empty segment is
/// dropped, so `f()` has no arguments and `f(a,)` has one).
///
/// Decides *where one argument ends and the next begins* and nothing else: it is
/// pure and infallible, so the scanner's state machine is no longer interleaved
/// with the fallible resolution of each argument.
fn split_top_level_args(args_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';
    let mut depth = 0;

    for ch in args_str.chars() {
        // Inside a quoted literal the only special character is the matching
        // closing quote; it closes the literal and is itself dropped, and
        // everything else — commas, parens — is taken verbatim.
        if in_quote {
            if ch == quote_char {
                in_quote = false;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_quote = true;
                quote_char = ch;
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' if depth > 0 => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }
    args
}

/// Resolve a single function argument: quoted literal, param ref, or nested function.
fn resolve_func_arg(
    arg: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<String, String> {
    // Quoted literal — strip quotes and return
    if (arg.starts_with('"') && arg.ends_with('"'))
        || (arg.starts_with('\'') && arg.ends_with('\''))
    {
        return Ok(arg[1..arg.len() - 1].to_string());
    }
    // Param reference — resolve from params
    if let Some(param_key) = arg.strip_prefix("params.") {
        return params
            .get(param_key)
            .map(yaml_value_to_string)
            .ok_or_else(|| format!("unknown param in function arg: {param_key}"));
    }
    // Machine reference
    if arg.starts_with("machine.") {
        let parts: Vec<&str> = arg.splitn(3, '.').collect();
        if parts.len() == 3 {
            let machine = machines
                .get(parts[1])
                .ok_or_else(|| format!("unknown machine: {}", parts[1]))?;
            return Ok(match parts[2] {
                "addr" => machine.addr.clone(),
                "hostname" => machine.hostname.clone(),
                "user" => machine.user.clone(),
                "arch" => machine.arch.clone(),
                _ => return Err(format!("unknown machine field: {}", parts[2])),
            });
        }
    }
    // Nested function call
    if arg.contains('(') {
        return resolve_function(arg, params, machines);
    }
    // Bare string (unquoted, no prefix) — treat as literal
    Ok(arg.to_string())
}

fn check_arg_count(func: &str, args: &[String], expected: usize) -> Result<(), String> {
    if args.len() != expected {
        return Err(format!(
            "{}() requires {} argument(s), got {}",
            func,
            expected,
            args.len()
        ));
    }
    Ok(())
}
