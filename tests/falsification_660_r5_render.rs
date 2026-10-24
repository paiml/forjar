//! forjar#660 round 5: the RENDER form of the loader. It keeps the stand-ins for
//! the template resolver, which restores them as its last step; a reader form
//! never carries one (`falsification_660_r5.rs`). One test per return path.

use forjar::core::resolver::{
    decode_literals, load_resolved_config, load_resolved_config_for_render,
    load_resolved_config_lenient, load_resolved_config_with, resolve_resource_templates,
};

fn fixture(value: &str, content: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let y = format!(
        "version: \"1.0\"\nname: r5r\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nparams:\n  p: PVAL\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/r5r.txt\n    content: \"{content}\"\ndata:\n  a:\n    type: command\n    value: \"printf '%s' '{value}'\"\n"
    );
    std::fs::write(d.path().join("f.yaml"), y).unwrap();
    d
}

fn rendered(cfg: &forjar::core::types::ForjarConfig) -> String {
    resolve_resource_templates(
        &cfg.resources["cfg"],
        &cfg.params,
        &cfg.data_values,
        &cfg.machines,
    )
    .unwrap()
    .content
    .unwrap()
}

#[test]
fn every_return_path_is_either_clean_or_renders_verbatim() {
    let (value, content, want) = ("{", "{{data.a}}{params.p}}", "{{params.p}}");
    let d = fixture(value, content);
    let f = d.path().join("f.yaml");
    let raw = |c: &forjar::core::types::ForjarConfig| c.resources["cfg"].content.clone().unwrap();

    // reader form: plain text, nothing to restore
    for (name, cfg) in [
        ("load_resolved_config", load_resolved_config(&f).unwrap()),
        ("lenient", load_resolved_config_lenient(&f).unwrap()),
    ] {
        assert_eq!(raw(&cfg), want, "{name}");
    }
    // render form: stand-ins held for the resolver, which restores them verbatim
    let for_render = load_resolved_config_for_render(&f).unwrap();
    let with = load_resolved_config_with(&f, |_| Ok(())).unwrap();
    for (name, cfg) in [("for_render", for_render), ("_with", with)] {
        assert!(
            raw(&cfg).contains('\u{E001}'),
            "{name} lost the guard: {:?}",
            raw(&cfg)
        );
        assert_eq!(rendered(&cfg), want, "{name}: re-expanded or not restored");
        let mut printed = cfg.clone();
        decode_literals(&mut printed).unwrap();
        assert_eq!(raw(&printed), want, "{name}: decode_literals");
    }
}
