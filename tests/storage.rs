use cable_management::{model::*,storage};
use std::{fs,path::PathBuf};
fn path(tag:&str)->PathBuf {std::env::temp_dir().join(format!("cable-test-{}-{tag}.json",std::process::id()))}

#[test]
fn json_roundtrip_preserves_model_ports_connections_and_views() {
    let p=Project::demo();let file=path("roundtrip");
    storage::save(&file,&p).unwrap();
    assert_eq!(storage::load(&file).unwrap(),p);
    fs::remove_file(file).unwrap();
}
#[test]
fn replacement_preserves_previous_version_in_backup() {
    let file=path("replace");let p=Project::demo();storage::save(&file,&p).unwrap();
    let mut next=p.clone();next.name="Nytt namn".into();storage::save(&file,&next).unwrap();
    assert_eq!(storage::load(&file).unwrap(),next);
    assert_eq!(storage::load(&storage::backup_path(&file)).unwrap(),p);
    fs::remove_file(&file).unwrap();fs::remove_file(storage::backup_path(&file)).unwrap();
}
#[test]
fn invalid_document_does_not_replace_previous_file() {
    let file=path("invalid");let p=Project::demo();storage::save(&file,&p).unwrap();
    let mut bad=p.clone();bad.connections[0].from.port=99999;
    assert!(storage::save(&file,&bad).is_err());assert_eq!(storage::load(&file).unwrap(),p);
    fs::write(&file,b"{\"version\":999}").unwrap();assert!(storage::load(&file).is_err());
    fs::remove_file(file).unwrap();
}
#[test]
fn save_to_missing_directory_returns_actionable_error() {
    let file=path("missing").join("project.json");
    let error=storage::save(&file,&Project::demo()).unwrap_err();
    assert!(error.contains("spar")||error.contains("skriv"),"{error}");
}
