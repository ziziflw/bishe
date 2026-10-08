use crate::{
    access::{self, Snapshot},
    repository::Repository,
};
use sha2::{Digest, Sha256};
use std::os::windows::fs::MetadataExt;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
struct Fixture {
    _temp: TempDir,
    source: PathBuf,
    file: PathBuf,
    database: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create isolated test directory");
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let file = source.join("测试 note.txt");
        fs::write(&file, b"private test contents\n").unwrap();
        let database = temp.path().join("application.sqlite3");
        Self {
            _temp: temp,
            source,
            file,
            database,
        }
    }
    fn db(&self) -> Repository {
        Repository::open(&self.database).unwrap()
    }
    fn register(&self, db: &mut Repository) -> i64 {
        db.register(&Snapshot::open_selected(&self.file, false).unwrap())
            .unwrap()
    }
}
fn signature(path: &Path) -> (Vec<u8>, u64, u64, u64) {
    let metadata = fs::metadata(path).unwrap();
    (
        Sha256::digest(fs::read(path).unwrap()).to_vec(),
        metadata.len(),
        metadata.last_write_time(),
        metadata.creation_time(),
    )
}
fn entries(path: &Path) -> Vec<PathBuf> {
    let mut v = fs::read_dir(path)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    v.sort();
    v
}
#[test]
fn import_metadata_duplicate_tags_search_and_restart_persistence() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    let record = db.file(id).unwrap();
    assert_eq!(record.name, "测试 note.txt");
    assert_eq!(record.extension, "txt");
    assert_eq!(record.path, f.file.to_str().unwrap());
    assert_eq!(record.size, 22);
    assert!(record.modified_at > 0 && record.registered_at > 0);
    assert_eq!(record.status, "READY");
    assert_eq!(f.register(&mut db), id);
    assert_eq!(db.search("", None, "").unwrap().len(), 1);
    db.create_tag("工作").unwrap();
    db.create_tag("Urgent").unwrap();
    let tag = db.tags().unwrap()[0].id;
    db.set_tag(id, tag, true).unwrap();
    db.set_tag(id, tag, true).unwrap();
    assert_eq!(db.file(id).unwrap().tags.len(), 1);
    db.rename_tag(tag, "项目").unwrap();
    assert_eq!(db.search("note", Some(tag), "txt").unwrap().len(), 1);
    assert!(db.search("' OR 1=1 --", None, "").unwrap().is_empty());
    assert!(db.search("", None, "pdf").unwrap().is_empty());
    drop(db);
    let db = f.db();
    assert_eq!(db.file(id).unwrap().tags[0].name, "项目");
    db.set_tag(id, tag, false).unwrap();
    assert!(db.file(id).unwrap().tags.is_empty());
    db.delete_index(id).unwrap();
    assert!(f.file.exists());
    assert!(db.search("", None, "").unwrap().is_empty());
}
#[test]
fn missing_and_same_identity_relink_preserve_labels() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    db.create_tag("保留").unwrap();
    db.set_tag(id, db.tags().unwrap()[0].id, true).unwrap();
    let moved = f.source.join("moved.txt");
    fs::rename(&f.file, &moved).unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "MISSING");
    assert_eq!(db.file(id).unwrap().tags.len(), 1);
    let different = f.source.join("different.txt");
    fs::write(&different, b"private test contents\n").unwrap();
    assert_eq!(
        db.relink(id, &Snapshot::open_selected(&different, false).unwrap())
            .unwrap_err()
            .code,
        "IDENTITY"
    );
    db.relink(id, &Snapshot::open_selected(&moved, false).unwrap())
        .unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "READY");
    assert_eq!(db.file(id).unwrap().tags.len(), 1);
    assert_eq!(db.search("", None, "").unwrap().len(), 1);
}
#[test]
fn replacement_must_not_inherit_labels() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    fs::rename(&f.file, f.source.join("original.txt")).unwrap();
    fs::write(&f.file, b"other").unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "CHANGED");
    assert_eq!(
        db.register(&Snapshot::open_selected(&f.file, false).unwrap())
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(db.search("", None, "").unwrap().len(), 1);
}
#[test]
fn revocation_prevents_read_even_when_file_missing_then_reauthorization() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    let grant = db.grants().unwrap()[0].id;
    db.revoke(grant).unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "UNAUTHORIZED");
    fs::rename(&f.file, f.source.join("moved.txt")).unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "UNAUTHORIZED");
    fs::rename(f.source.join("moved.txt"), &f.file).unwrap();
    f.register(&mut db);
    assert_eq!(db.refresh(id).unwrap().status, "READY");
}
#[test]
fn directory_grant_revocation_and_outside_paths() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    db.revoke(db.grants().unwrap()[0].id).unwrap();
    db.grant_directory(&Snapshot::open_selected(&f.source, true).unwrap())
        .unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "READY");
    let outside = f._temp.path().join("source-other");
    fs::create_dir(&outside).unwrap();
    let other = outside.join("x.txt");
    fs::write(&other, b"x").unwrap();
    let e = access::authorized_snapshot(&other, "irrelevant", &db.grants().unwrap())
        .err()
        .unwrap();
    assert_eq!(e.code, "UNAUTHORIZED");
    let traversal = f.source.join("..\\source-other\\x.txt");
    assert!(access::authorized_snapshot(&traversal, "x", &db.grants().unwrap()).is_err());
    for grant in db.grants().unwrap() {
        db.revoke(grant.id).unwrap();
    }
    assert_eq!(db.refresh(id).unwrap().status, "UNAUTHORIZED");
}
#[test]
fn replaced_directory_grant_is_rejected() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    db.revoke(db.grants().unwrap()[0].id).unwrap();
    db.grant_directory(&Snapshot::open_selected(&f.source, true).unwrap())
        .unwrap();
    fs::rename(&f.source, f._temp.path().join("old-source")).unwrap();
    fs::create_dir(&f.source).unwrap();
    assert_eq!(db.refresh(id).unwrap().status, "UNAUTHORIZED");
}
#[test]
fn write_failure_rolls_back_and_never_changes_sources_or_creates_sidecars() {
    let f = Fixture::new();
    let before = signature(&f.file);
    let listing = entries(&f.source);
    let mut db = f.db();
    db.conn.execute_batch("CREATE TRIGGER injected_failure BEFORE INSERT ON file_records BEGIN SELECT RAISE(ABORT, 'test disk failure'); END;").unwrap();
    assert!(db
        .register(&Snapshot::open_selected(&f.file, false).unwrap())
        .is_err());
    assert!(db.grants().unwrap().is_empty());
    assert!(db.search("", None, "").unwrap().is_empty());
    assert_eq!(signature(&f.file), before);
    assert_eq!(entries(&f.source), listing);
    db.conn
        .execute_batch("DROP TRIGGER injected_failure;")
        .unwrap();
    let id = f.register(&mut db);
    db.conn.execute_batch("PRAGMA query_only=ON;").unwrap();
    assert!(db.create_tag("blocked").is_err());
    assert_eq!(db.file(id).unwrap().status, "READY");
    assert_eq!(signature(&f.file), before);
    assert_eq!(entries(&f.source), listing);
}
#[test]
fn link_jump_rejected_and_locked_handles_prevent_replacement() {
    let f = Fixture::new();
    let junction = f._temp.path().join("junction");
    // Fixed test-only command, paths generated by TempDir, never a production shell interface.
    let output = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&f.source)
        .output()
        .unwrap();
    assert!(output.status.success(), "test junction creation failed");
    assert_eq!(
        Snapshot::open_selected(&junction.join("测试 note.txt"), false)
            .err()
            .unwrap()
            .code,
        "LINK"
    );
    fs::remove_dir(&junction).unwrap();
    let snapshot = Snapshot::open_selected(&f.file, false).unwrap();
    assert!(fs::rename(&f.file, f.source.join("blocked.txt")).is_err());
    assert!(fs::OpenOptions::new().write(true).open(&f.file).is_err());
    drop(snapshot);
    assert!(fs::rename(&f.file, f.source.join("allowed.txt")).is_ok());
}
#[test]
fn conflicting_writer_is_reported_without_mutating_index() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    let _writer = fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&f.file)
        .unwrap();
    assert!(db.refresh(id).is_err());
    assert_eq!(db.file(id).unwrap().status, "ERROR");
}
#[test]
fn input_constraints_and_migration_version_preserve_database() {
    let f = Fixture::new();
    let db = f.db();
    assert!(db.create_tag("").is_err());
    assert!(db.create_tag("\n").is_err());
    db.create_tag("Test").unwrap();
    assert!(db.create_tag("test").is_err());
    assert!(db.set_tag(-1, 1, true).is_err());
    assert!(db.set_tag(900, 800, true).is_err());
    db.conn.execute_batch("PRAGMA user_version=99;").unwrap();
    drop(db);
    assert!(Repository::open(&f.database).is_err());
    let c = rusqlite::Connection::open(&f.database).unwrap();
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM tags", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn multiple_tags_and_transaction_failure_preserve_existing_associations() {
    let f = Fixture::new();
    let mut db = f.db();
    let id = f.register(&mut db);
    db.create_tag("alpha").unwrap();
    db.create_tag("beta").unwrap();
    let tags = db.tags().unwrap();
    for tag in &tags {
        db.set_tag(id, tag.id, true).unwrap();
    }
    assert_eq!(db.file(id).unwrap().tags.len(), 2);
    db.conn.execute_batch("CREATE TRIGGER injected_update_failure BEFORE UPDATE ON file_records BEGIN SELECT RAISE(ABORT, 'test update failure'); END;").unwrap();
    assert!(db
        .register(&Snapshot::open_selected(&f.file, false).unwrap())
        .is_err());
    assert_eq!(db.file(id).unwrap().tags.len(), 2);
    db.set_tag(id, tags[0].id, false).unwrap();
    assert_eq!(db.file(id).unwrap().tags.len(), 1);
}
