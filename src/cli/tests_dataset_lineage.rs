//! Tests: FJ-1413 dataset versioning and lineage.

#![allow(unused_imports)]
use super::commands::*;
use super::dataset_lineage::*;
use super::dispatch::*;
use super::helpers::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let file = dir.join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();
        file
    }

    #[test]
    fn test_fj1413_lineage_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_dataset_lineage(&file, false).unwrap();
    }

    #[test]
    fn test_fj1413_lineage_with_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("input.csv"), "id,val\n1,2\n").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: data-pipe
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  ingest:
    type: file
    machine: m
    path: /data/input.csv
    source: input.csv
    tags: [data, ingest]
    output_artifacts:
      - input.csv
  transform:
    type: task
    machine: m
    command: "python transform.py"
    depends_on: [ingest]
    tags: [data, transform]
    output_artifacts:
      - output.parquet
"#,
        );
        cmd_dataset_lineage(&file, false).unwrap();
    }

    #[test]
    fn test_fj1413_lineage_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        cmd_dataset_lineage(&file, true).unwrap();
    }

    #[test]
    fn test_fj1413_lineage_by_group() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: grouped
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  stage1:
    type: task
    machine: m
    command: "process.sh"
    resource_group: data-pipeline
    output_artifacts:
      - stage1.out
"#,
        );
        cmd_dataset_lineage(&file, false).unwrap();
    }

    #[test]
    fn test_fj1413_lineage_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::DatasetLineage(DatasetLineageArgs {
                file,
                json: false,
            }),
            false,
            true,
        )
        .unwrap();
    }
}
