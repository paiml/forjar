use std::collections::BTreeMap;

#[derive(serde::Deserialize, Default)]
struct ContractYaml {
    #[serde(default)]
    equations: BTreeMap<String, EquationYaml>,
}

#[derive(serde::Deserialize, Default)]
struct EquationYaml {
    #[serde(default)]
    preconditions: Vec<String>,
    #[serde(default)]
    postconditions: Vec<String>,
    #[serde(default)]
    invariants: Vec<String>,
}

fn emit_contract_assertions() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
    if !dir.exists() {
        return;
    }
    println!("cargo::rerun-if-changed=contracts/");

    let mut count = 0usize;
    for entry in std::fs::read_dir(&dir).expect("read contracts/") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .replace('-', "_");

        let content = std::fs::read_to_string(&path).expect("read contract yaml");
        let contract: ContractYaml =
            serde_yaml_ng::from_str(&content).expect("parse contract yaml");

        for (eq_name, eq) in &contract.equations {
            let key = format!("{}_{}", stem, eq_name).to_uppercase();
            for (i, pre) in eq.preconditions.iter().enumerate() {
                println!("cargo::rustc-env=CONTRACT_PRE_{key}_{i}={pre}");
                count += 1;
            }
            for (i, post) in eq.postconditions.iter().enumerate() {
                println!("cargo::rustc-env=CONTRACT_POST_{key}_{i}={post}");
                count += 1;
            }
            for (i, inv) in eq.invariants.iter().enumerate() {
                println!("cargo::rustc-env=CONTRACT_INV_{key}_{i}={inv}");
                count += 1;
            }
        }
    }
    eprintln!("forjar build.rs: emitted {count} contract env vars");
}

/// One entry of `contracts/binding.yaml`: the claim that some Rust item
/// implements a named equation of a named contract.
#[derive(serde::Deserialize)]
struct BindingYaml {
    contract: String,
    equation: String,
}

#[derive(serde::Deserialize)]
struct BindingRegistryYaml {
    #[serde(default)]
    bindings: Vec<BindingYaml>,
}

/// GH-298: make "N/N bindings bound" mean RESOLVED, not DECLARED.
///
/// `provable_contracts::build_helper::verify_bindings` reads `status:` out of
/// binding.yaml and nothing else. It never opens a contract file, so a binding
/// could name a contract that does not exist or an equation that contract does
/// not define and still be counted. One did: an `apply-receipt-v1.yaml` entry
/// claimed to implement `receipt_deletion`, an equation that contract has never
/// declared, and the build printed "43/43 bindings bound" for months.
///
/// This resolves the other half of every binding. It is duplicated as a
/// `#[test]` in tests/falsification_contract_citations_resolve.rs, deliberately:
/// a build result is cacheable and a test result is not.
/// The reason this binding does not resolve, or `None` if it does.
fn unresolved_binding(dir: &std::path::Path, b: &BindingYaml) -> Option<String> {
    let Ok(body) = std::fs::read_to_string(dir.join(&b.contract)) else {
        return Some(format!(
            "binding for `{}` names {}, which does not exist",
            b.equation, b.contract
        ));
    };
    let contract: ContractYaml = serde_yaml_ng::from_str(&body).expect("parse contract yaml");
    if contract.equations.contains_key(&b.equation) {
        return None;
    }
    Some(format!(
        "{} does not define equation `{}`, but a binding claims to implement it",
        b.contract, b.equation
    ))
}

/// GH-298: make "N/N bindings bound" mean RESOLVED, not DECLARED.
///
/// `provable_contracts::build_helper::verify_bindings` reads `status:` out of
/// binding.yaml and nothing else. It never opens a contract file, so a binding
/// could name a contract that does not exist or an equation that contract does
/// not define and still be counted. One did: an `apply-receipt-v1.yaml` entry
/// claimed to implement `receipt_deletion`, an equation that contract has never
/// declared, and the build printed "43/43 bindings bound" for months.
///
/// This resolves the other half of every binding. It is duplicated as a
/// `#[test]` in tests/falsification_contract_citations_resolve.rs, deliberately:
/// a build result is cacheable and a test result is not.
fn verify_binding_equations(binding_path: &std::path::Path) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
    let text = std::fs::read_to_string(binding_path).expect("read binding.yaml");
    let registry: BindingRegistryYaml = serde_yaml_ng::from_str(&text).expect("parse binding.yaml");
    assert!(
        !registry.bindings.is_empty(),
        "contracts/binding.yaml declares no bindings — an empty registry is a \
         finding, not a pass"
    );
    let unresolved: Vec<String> = registry
        .bindings
        .iter()
        .filter_map(|b| unresolved_binding(&dir, b))
        .collect();
    assert!(
        unresolved.is_empty(),
        "bindings counted as bound without being resolved:\n  {}",
        unresolved.join("\n  ")
    );
    eprintln!(
        "forjar build.rs: {} binding(s) resolved to a declared equation",
        registry.bindings.len()
    );
}

fn main() {
    emit_contract_assertions();

    let binding_path = "contracts/binding.yaml";
    if std::path::Path::new(binding_path).exists() {
        provable_contracts::build_helper::verify_bindings(
            binding_path,
            provable_contracts::build_helper::BindingPolicy::AllImplemented,
        );
        verify_binding_equations(std::path::Path::new(binding_path));
    }
}
