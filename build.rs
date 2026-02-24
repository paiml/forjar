fn main() {
    let binding_path = "../provable-contracts/contracts/forjar/binding.yaml";
    if std::path::Path::new(binding_path).exists() {
        provable_contracts::build_helper::verify_bindings(
            binding_path,
            provable_contracts::build_helper::BindingPolicy::WarnOnGaps,
        );
    }
}
