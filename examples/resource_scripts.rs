//! Show all three script types (check, apply, state_query) for each resource type.
//!
//! Each resource handler generates three shell scripts:
//! - check: determine if the resource exists / matches desired state
//! - apply: converge the resource to desired state
//! - state_query: gather state for BLAKE3 hashing (drift detection)
//!
//! Usage: cargo run --example resource_scripts

use forjar::core::codegen;
use forjar::core::types::{MachineTarget, Resource, ResourceType};

fn make_resource(rt: ResourceType) -> Resource {
    Resource {
        resource_type: rt,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: None,
        content: None,
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: std::collections::HashMap::new(),
        arch: vec![],
        tags: vec![],
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
    }
}

fn print_scripts(label: &str, resource: &Resource) {
    println!("╔══ {} ══╗", label);

    println!("\n── check ──");
    println!("{}", codegen::check_script(resource).unwrap());

    println!("\n── apply ──");
    println!("{}", codegen::apply_script(resource).unwrap());

    println!("\n── state_query ──");
    println!("{}", codegen::state_query_script(resource).unwrap());

    println!();
}

fn main() {
    // Package: apt install nginx
    let mut pkg = make_resource(ResourceType::Package);
    pkg.provider = Some("apt".to_string());
    pkg.packages = vec!["nginx".to_string(), "certbot".to_string()];
    print_scripts("Package (apt)", &pkg);

    // File: managed config file
    let mut file = make_resource(ResourceType::File);
    file.path = Some("/etc/nginx/nginx.conf".to_string());
    file.content = Some("events { worker_connections 1024; }".to_string());
    file.owner = Some("root".to_string());
    file.group = Some("root".to_string());
    file.mode = Some("0644".to_string());
    print_scripts("File (config)", &file);

    // Service: nginx running
    let mut svc = make_resource(ResourceType::Service);
    svc.name = Some("nginx".to_string());
    svc.state = Some("running".to_string());
    svc.enabled = Some(true);
    print_scripts("Service", &svc);

    // Mount: NFS share
    let mut mnt = make_resource(ResourceType::Mount);
    mnt.source = Some("nfs.local:/exports/data".to_string());
    mnt.path = Some("/mnt/data".to_string());
    mnt.fs_type = Some("nfs".to_string());
    mnt.options = Some("rw,soft".to_string());
    print_scripts("Mount (NFS)", &mnt);

    // User: deploy user with SSH keys
    let mut user = make_resource(ResourceType::User);
    user.name = Some("deploy".to_string());
    user.shell = Some("/bin/bash".to_string());
    user.groups = vec!["docker".to_string(), "sudo".to_string()];
    user.ssh_authorized_keys = vec!["ssh-ed25519 AAAA... deploy@host".to_string()];
    print_scripts("User", &user);

    // Docker: container with ports and volumes
    let mut docker = make_resource(ResourceType::Docker);
    docker.name = Some("web-app".to_string());
    docker.image = Some("myapp:v2".to_string());
    docker.ports = vec!["8080:80".to_string()];
    docker.environment = vec!["ENV=production".to_string()];
    docker.volumes = vec!["/data:/app/data".to_string()];
    docker.restart = Some("unless-stopped".to_string());
    print_scripts("Docker", &docker);

    // Cron: scheduled job
    let mut cron = make_resource(ResourceType::Cron);
    cron.name = Some("backup".to_string());
    cron.schedule = Some("0 2 * * *".to_string());
    cron.command = Some("/usr/local/bin/backup.sh".to_string());
    print_scripts("Cron", &cron);

    // Network: firewall rule
    let mut net = make_resource(ResourceType::Network);
    net.port = Some("443".to_string());
    net.protocol = Some("tcp".to_string());
    net.action = Some("allow".to_string());
    print_scripts("Network (ufw)", &net);

    // Pepita: kernel namespace isolation
    let mut pepita = make_resource(ResourceType::Pepita);
    pepita.name = Some("sandbox".to_string());
    pepita.state = Some("present".to_string());
    pepita.chroot_dir = Some("/var/sandbox".to_string());
    pepita.netns = true;
    pepita.seccomp = true;
    pepita.cpuset = Some("0-3".to_string());
    pepita.memory_limit = Some(536870912); // 512 MiB
    pepita.overlay_lower = Some("/base".to_string());
    pepita.overlay_upper = Some("/var/sandbox/upper".to_string());
    pepita.overlay_work = Some("/var/sandbox/work".to_string());
    pepita.overlay_merged = Some("/var/sandbox/merged".to_string());
    print_scripts("Pepita (kernel isolation)", &pepita);

    println!("All 9 resource types × 3 script types demonstrated.");
}
