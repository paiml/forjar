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

fn main() {
    emit_contract_assertions();

    let binding_path = "contracts/binding.yaml";
    if std::path::Path::new(binding_path).exists() {
        provable_contracts::build_helper::verify_bindings(
            binding_path,
            provable_contracts::build_helper::BindingPolicy::AllImplemented,
        );
    }
}
