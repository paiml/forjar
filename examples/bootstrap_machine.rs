//! FJ-49: Bootstrap machine workflow — SSH key injection + sudo setup.
//!
//! Shows the bootstrap pipeline phases and how forjar.yaml machine config
//! drives the provisioning sequence. Does NOT require real SSH connectivity.
//!
//! Usage: cargo run --example bootstrap_machine

use forjar::core::types::Machine;

fn main() {
    println!("=== FJ-49: Bootstrap Machine Workflow ===\n");

    let machine = Machine {
        hostname: "yoga".to_string(),
        addr: "192.168.1.100".to_string(),
        user: "noah".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: Some("~/.ssh/id_ed25519".to_string()),
        roles: vec!["compute".to_string()],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    println!("Machine config:");
    println!("  hostname: {}", machine.hostname);
    println!("  addr:     {}", machine.addr);
    println!("  user:     {}", machine.user);
    println!("  arch:     {}", machine.arch);
    println!(
        "  ssh_key:  {}",
        machine.ssh_key.as_deref().unwrap_or("(default)")
    );

    println!("\n--- Bootstrap phases ---\n");

    // Phase 1: SSH key injection
    println!("Phase 1: SSH key injection");
    let pub_key = format!(
        "{}.pub",
        machine.ssh_key.as_deref().unwrap_or("~/.ssh/id_ed25519")
    );
    println!(
        "  ssh-copy-id -i {pub_key} {}@{}",
        machine.user, machine.addr
    );
    println!("  (with --password: uses sshpass for non-interactive copy)");

    // Phase 2: Sudo configuration
    println!("\nPhase 2: Passwordless sudo");
    let sudoers_line = format!("{} ALL=(ALL) NOPASSWD:ALL", machine.user);
    let sudoers_file = format!("/etc/sudoers.d/{}-nopasswd", machine.user);
    println!("  echo '{sudoers_line}' > {sudoers_file}");
    println!("  chmod 0440 {sudoers_file}");

    // Phase 3: Verification
    println!("\nPhase 3: Verification");
    println!(
        "  ssh -o BatchMode=yes {}@{} true   # key auth",
        machine.user, machine.addr
    );
    println!(
        "  ssh -o BatchMode=yes {}@{} sudo -n true   # sudo",
        machine.user, machine.addr
    );

    // Show the CLI command
    println!("\n--- CLI command ---\n");
    println!("  forjar bootstrap -f forjar.yaml --machine yoga");
    println!("  forjar bootstrap -f forjar.yaml --machine yoga --password  # with password");

    // Show how image command eliminates bootstrap
    println!("\n--- FJ-52: Image eliminates bootstrap ---\n");
    println!("  forjar image generates ISOs with SSH keys + sudo pre-configured.");
    println!("  Machines booted from autoinstall ISOs skip bootstrap entirely.");

    // Tilde expansion used by both bootstrap and image
    println!("\n--- Tilde expansion ---\n");
    let expanded = forjar::cli::image_cmd::expand_tilde("~/.ssh/id_ed25519.pub");
    println!("  expand_tilde(\"~/.ssh/id_ed25519.pub\") = \"{expanded}\"");

    // Show SSH key reading
    println!("\n--- SSH key reading ---\n");
    let keys = forjar::cli::image_cmd::read_ssh_pub_key(None);
    println!("  read_ssh_pub_key(None) = {:?} (no key configured)", keys);

    println!("\nBootstrap complete — machine ready for `forjar apply`.");
}
