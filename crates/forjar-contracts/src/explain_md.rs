//! Markdown rendering helpers for contract explanation output.
//!
//! Extracted from `explain.rs` to keep each file under 500 lines and each
//! function concise.

use std::fmt::Write;

use crate::binding::{BindingRegistry, ImplStatus};
use crate::proof_status::compute_proof_level;
use crate::schema::{Contract, Equation};

pub(super) fn write_md_header(out: &mut String, contract: &Contract, stem: &str) {
    let _ = writeln!(out, "# {stem}\n");
    let _ = writeln!(
        out,
        "**Version:** {} | **Description:** {}\n",
        contract.metadata.version, contract.metadata.description
    );
}

pub(super) fn write_md_references(out: &mut String, contract: &Contract) {
    if contract.metadata.references.is_empty() {
        return;
    }
    let _ = writeln!(out, "## References\n");
    for r in &contract.metadata.references {
        let _ = writeln!(out, "- {r}");
    }
    let _ = writeln!(out);
}

pub(super) fn write_md_equations(out: &mut String, contract: &Contract) {
    if contract.equations.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Equations\n");
    for (name, eq) in &contract.equations {
        write_md_equation(out, name, eq);
    }
}

fn write_md_equation(out: &mut String, name: &str, eq: &Equation) {
    let _ = writeln!(out, "### {name}\n");
    let latex = crate::latex::math_to_latex(&eq.formula);
    let _ = writeln!(out, "$$\n{latex}\n$$\n");
    if let Some(ref dom) = eq.domain {
        let _ = writeln!(out, "**Domain:** ${}$\n", crate::latex::math_to_latex(dom));
    }
    if let Some(ref cod) = eq.codomain {
        let _ = writeln!(
            out,
            "**Codomain:** ${}$\n",
            crate::latex::math_to_latex(cod)
        );
    }
    if !eq.invariants.is_empty() {
        let _ = writeln!(out, "**Invariants:**\n");
        for inv in &eq.invariants {
            let _ = writeln!(out, "- ${}$", crate::latex::math_to_latex(inv));
        }
        let _ = writeln!(out);
    }
    write_md_code_list(out, "Preconditions", &eq.preconditions);
    write_md_code_list(out, "Postconditions", &eq.postconditions);
}

fn write_md_code_list(out: &mut String, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "**{heading}:**\n");
    for item in items {
        let _ = writeln!(out, "- `{item}`");
    }
    let _ = writeln!(out);
}

pub(super) fn write_md_obligations(out: &mut String, contract: &Contract) {
    if contract.proof_obligations.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Proof Obligations\n");
    let _ = writeln!(out, "| # | Type | Property | Formal |");
    let _ = writeln!(out, "|---|------|----------|--------|");
    for (i, ob) in contract.proof_obligations.iter().enumerate() {
        let formal = ob.formal.as_deref().unwrap_or("");
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | {} |",
            i + 1,
            ob.obligation_type,
            ob.property,
            formal
        );
    }
    let _ = writeln!(out);
}

pub(super) fn write_md_verification(out: &mut String, contract: &Contract) {
    let level = compute_proof_level(contract, None);
    let _ = writeln!(out, "## Verification\n");
    let _ = writeln!(out, "**Proof level:** {level}\n");
    if let Some(ref vs) = contract.verification_summary {
        let _ = writeln!(
            out,
            "- Lean: {}/{} proved",
            vs.l4_lean_proved, vs.total_obligations
        );
    }
    let _ = writeln!(out, "- Kani: {} harnesses", contract.kani_harnesses.len());
    let _ = writeln!(
        out,
        "- Tests: {} falsification\n",
        contract.falsification_tests.len()
    );
}

pub(super) fn write_md_bindings(
    out: &mut String,
    contract: &Contract,
    stem: &str,
    binding: Option<&BindingRegistry>,
) {
    let Some(registry) = binding else {
        return;
    };
    let contract_file = format!("{stem}.yaml");
    if !has_contract_bindings(contract, registry, &contract_file) {
        return;
    }
    let _ = writeln!(out, "## Bindings ({})\n", registry.target_crate);
    let _ = writeln!(out, "| Equation | Status |");
    let _ = writeln!(out, "|----------|--------|");
    for eq_name in contract.equations.keys() {
        let status = find_equation_status(registry, &contract_file, eq_name);
        let _ = writeln!(out, "| {eq_name} | {status} |");
    }
    let _ = writeln!(out);
}

fn has_contract_bindings(
    contract: &Contract,
    registry: &BindingRegistry,
    contract_file: &str,
) -> bool {
    contract.equations.keys().any(|eq| {
        registry
            .bindings
            .iter()
            .any(|b| b.contract == contract_file && b.equation == *eq)
    })
}

fn find_equation_status(
    registry: &BindingRegistry,
    contract_file: &str,
    eq_name: &str,
) -> &'static str {
    registry
        .bindings
        .iter()
        .find(|b| b.contract == contract_file && b.equation == *eq_name)
        .map_or("missing", |b| impl_status_label(b.status))
}

fn impl_status_label(status: ImplStatus) -> &'static str {
    match status {
        ImplStatus::Implemented => "implemented",
        ImplStatus::Partial => "partial",
        ImplStatus::NotImplemented => "not_implemented",
        ImplStatus::Pending => "pending",
    }
}
