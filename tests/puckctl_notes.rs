use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct TestDocument(PathBuf);

impl TestDocument {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("puckctl-{}.puck", uuid::Uuid::now_v7())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDocument {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn puckctl(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_puckctl"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn notes_can_be_added_listed_and_read() {
    let document = TestDocument::new();
    let path = document.path().to_str().unwrap();

    assert!(puckctl(&["document", "new", path]).status.success());

    let first = puckctl(&["document", "note", "add", path, "alpha-01 is 192.168.1.10"]);
    assert!(first.status.success());
    let first_id = String::from_utf8(first.stdout).unwrap().trim().to_owned();

    let second = puckctl(&["document", "note", "add", path, "alpha-02"]);
    assert!(second.status.success());
    let second_id = String::from_utf8(second.stdout).unwrap().trim().to_owned();

    let list = puckctl(&["document", "note", "list", path]);
    assert!(list.status.success());
    let list = String::from_utf8(list.stdout).unwrap();
    assert!(
        list.lines()
            .any(|line| line.starts_with(&format!("{first_id}\t1\t")))
    );
    assert!(
        list.lines()
            .any(|line| line.ends_with("\talpha-01 is 192.168.1.10"))
    );
    assert!(
        list.lines()
            .any(|line| line.starts_with(&format!("{second_id}\t1\t")))
    );
    assert!(list.lines().any(|line| line.ends_with("\talpha-02")));

    let read = puckctl(&["document", "note", "read", path, &first_id]);
    assert!(read.status.success());
    assert_eq!(read.stdout, b"alpha-01 is 192.168.1.10");

    assert!(
        !puckctl(&["document", "note", "read", path, "not-a-uuid"])
            .status
            .success()
    );
    assert!(
        !puckctl(&[
            "document",
            "note",
            "read",
            path,
            &uuid::Uuid::now_v7().to_string(),
        ])
        .status
        .success()
    );
}
