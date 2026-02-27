use crate::core::types::*;
use std::collections::HashMap;

/// Resolve a template function call like `upper(params.name)`.
pub(crate) fn resolve_function(
    expr: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<String, String> {
    let open_paren = expr
        .find('(')
        .ok_or_else(|| format!("malformed function: {}", expr))?;
    if !expr.ends_with(')') {
        return Err(format!("unclosed parenthesis in function: {}", expr));
    }
    let func_name = expr[..open_paren].trim();
    let args_str = &expr[open_paren + 1..expr.len() - 1];
    let args = parse_func_args(args_str, params, machines)?;

    match func_name {
        "upper" => {
            check_arg_count("upper", &args, 1)?;
            Ok(args[0].to_uppercase())
        }
        "lower" => {
            check_arg_count("lower", &args, 1)?;
            Ok(args[0].to_lowercase())
        }
        "trim" => {
            check_arg_count("trim", &args, 1)?;
            Ok(args[0].trim().to_string())
        }
        "default" => {
            check_arg_count("default", &args, 2)?;
            Ok(if args[0].is_empty() {
                args[1].clone()
            } else {
                args[0].clone()
            })
        }
        "replace" => {
            check_arg_count("replace", &args, 3)?;
            Ok(args[0].replace(args[1].as_str(), args[2].as_str()))
        }
        "env" => {
            check_arg_count("env", &args, 1)?;
            std::env::var(&args[0]).map_err(|_| format!("env var '{}' not set", args[0]))
        }
        "b3sum" => {
            check_arg_count("b3sum", &args, 1)?;
            Ok(blake3::hash(args[0].as_bytes()).to_hex().to_string())
        }
        "join" => {
            check_arg_count("join", &args, 2)?;
            // First arg is a comma-separated list, second is the new separator
            let parts: Vec<&str> = args[0].split(',').map(|s| s.trim()).collect();
            Ok(parts.join(&args[1]))
        }
        "split" => {
            check_arg_count("split", &args, 2)?;
            // Split string by delimiter, return comma-separated
            let parts: Vec<&str> = args[0].split(args[1].as_str()).collect();
            Ok(parts.join(","))
        }
        _ => Err(format!("unknown template function: {}", func_name)),
    }
}

/// Parse function arguments, resolving param/machine references and quoted literals.
fn parse_func_args(
    args_str: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';
    let mut depth = 0;

    for ch in args_str.chars() {
        match ch {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = ch;
            }
            c if in_quote && c == quote_char => {
                in_quote = false;
            }
            '(' if !in_quote => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_quote && depth > 0 => {
                depth -= 1;
                current.push(ch);
            }
            ',' if !in_quote && depth == 0 => {
                args.push(resolve_func_arg(current.trim(), params, machines)?);
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        args.push(resolve_func_arg(trimmed, params, machines)?);
    }
    Ok(args)
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
            .ok_or_else(|| format!("unknown param in function arg: {}", param_key));
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
