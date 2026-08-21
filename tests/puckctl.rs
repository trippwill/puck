// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;
use std::process::{Command, Output};

struct TestDocument(PathBuf);

impl TestDocument {
    fn new() -> Self {
        let document =
            Self(std::env::temp_dir().join(format!("puckctl-{}.puck", uuid::Uuid::now_v7())));
        document.success(&["new"]);
        document
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_puckctl"))
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn success(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn text(&self, args: &[&str]) -> String {
        String::from_utf8(self.success(args).stdout).unwrap()
    }

    fn id(&self, args: &[&str]) -> String {
        let id = self.text(args).trim().to_owned();
        id.parse::<uuid::Uuid>().unwrap();
        id
    }

    fn failure(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(output.status.code().is_some_and(|code| code != 0));
        String::from_utf8(output.stderr).unwrap()
    }

    fn count(&self, table: &str, deleted: Option<bool>) -> i64 {
        let conn = tokio_rusqlite::rusqlite::Connection::open(&self.0).unwrap();
        let filter = deleted.map_or(String::new(), |deleted| {
            format!(" WHERE deleted = {}", i64::from(deleted))
        });
        conn.query_row(
            &format!("SELECT count(*) FROM {table}{filter}"),
            [],
            |row| row.get(0),
        )
        .unwrap()
    }
}

impl Drop for TestDocument {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn assert_not_found(error: &str, kind: &str, id: &str) {
    assert!(
        error.contains(kind) && error.contains(id) && error.contains("not found"),
        "{error:?}"
    );
}

#[test]
fn notes_can_be_managed() {
    let document = TestDocument::new();
    let first = document.id(&["note", "add", "alpha-01 is 192.168.1.10"]);
    let second = document.id(&["note", "add", "alpha-02"]);

    let list = document.text(&["note", "list"]);
    assert!(
        list.lines()
            .any(|line| line.starts_with(&format!("{first}\t1\t")))
    );
    assert!(
        list.lines()
            .any(|line| line.ends_with("\talpha-01 is 192.168.1.10"))
    );
    assert!(
        list.lines()
            .any(|line| line.starts_with(&format!("{second}\t1\t")))
    );
    assert!(list.lines().any(|line| line.ends_with("\talpha-02")));
    assert_eq!(
        document.text(&["note", "read", &first]),
        "alpha-01 is 192.168.1.10"
    );

    document.success(&["note", "edit", &first, "alpha-01 moved to 192.168.1.11"]);
    assert_eq!(
        document.text(&["note", "read", &first]),
        "alpha-01 moved to 192.168.1.11"
    );
    let list = document.text(&["note", "list"]);
    assert!(
        list.lines()
            .next()
            .unwrap()
            .starts_with(&format!("{first}\t2\t"))
    );

    document.success(&["note", "archive", &first]);
    assert!(!document.text(&["note", "list"]).contains(&first));
    assert!(
        document
            .text(&["note", "list", "--archived"])
            .lines()
            .any(|line| line.starts_with(&format!("{first}\t2\t")))
    );
    assert_eq!(
        document.text(&["note", "read", "--archived", &first]),
        "alpha-01 moved to 192.168.1.11"
    );

    document.success(&["note", "unarchive", &first]);
    assert!(document.text(&["note", "list"]).contains(&first));
    assert!(
        !document
            .text(&["note", "list", "--archived"])
            .contains(&first)
    );

    document.success(&["note", "archive", &first]);
    document.success(&["note", "delete", &first]);
    assert!(
        !document
            .text(&["note", "list", "--archived"])
            .contains(&first)
    );
    assert!(
        document
            .text(&["note", "list", "--deleted"])
            .contains(&first)
    );
    assert_eq!(document.count("notes", Some(true)), 1);
    document.success(&["note", "undelete", &first]);
    assert!(
        document
            .text(&["note", "list", "--archived"])
            .contains(&first)
    );
    assert!(document.text(&["note", "list", "--deleted"]).is_empty());
    document.success(&["note", "delete", &first]);
    document.success(&["clean"]);
    assert_eq!(document.count("notes", None), 1);
}

#[test]
fn note_failures_are_reported() {
    let document = TestDocument::new();
    let note = document.id(&["note", "add", "alpha"]);
    let missing = uuid::Uuid::now_v7().to_string();

    assert!(
        document
            .failure(&["note", "read", "not-a-uuid"])
            .contains("invalid value")
    );
    assert_not_found(
        &document.failure(&["note", "read", &missing]),
        "Note",
        &missing,
    );
    assert_not_found(
        &document.failure(&["note", "edit", &missing, "missing"]),
        "Note",
        &missing,
    );
    assert_not_found(
        &document.failure(&["note", "delete", &note]),
        "Archived note",
        &note,
    );

    document.success(&["note", "archive", &note]);
    assert_not_found(
        &document.failure(&["note", "archive", &note]),
        "Note",
        &note,
    );
    document.success(&["note", "unarchive", &note]);
    assert_not_found(
        &document.failure(&["note", "unarchive", &note]),
        "Note",
        &note,
    );
}

#[test]
fn active_notes_can_be_searched() {
    let document = TestDocument::new();
    document.success(&["note", "add", "Unrelated"]);
    let matching = document.id(&["note", "add", "Router\nFind alpha-01, café, 🦀 here."]);

    let search = document.text(&["note", "search", "alpha-01, café, 🦀"]);
    assert_eq!(search.lines().count(), 1);
    assert!(search.starts_with(&format!("{matching}\t1\t")));
    assert!(search.trim_end().ends_with("\tRouter"));
    assert!(document.text(&["note", "search", "missing"]).is_empty());

    document.success(&["note", "archive", &matching]);
    assert!(document.text(&["note", "search", "alpha-01"]).is_empty());
}

#[test]
fn structured_data_can_be_managed() {
    let document = TestDocument::new();
    let collection = document.id(&["collection", "add", "Old name"]);
    assert_eq!(
        document.text(&["collection", "read", &collection]),
        "Old name"
    );
    document.success(&["collection", "rename", &collection, "Values"]);
    assert_eq!(
        document.text(&["collection", "list"]),
        format!("{collection}\tValues\n")
    );

    let record = document.id(&["record", "add", &collection]);
    let record_row = format!("{record}\t{collection}");
    assert_eq!(document.text(&["record", "read", &record]), record_row);
    assert_eq!(
        document.text(&["record", "list", &collection]),
        format!("{record_row}\n")
    );

    let cases = [
        ("text", "Text", "first", "line\nwith\ttab\\slash"),
        ("boolean", "Boolean", "true", "false"),
        ("integer", "Integer", "-42", "99"),
        ("date", "Date", "2026-08-20", "2026-08-21"),
        ("time", "Time", "01:02:03", "23:59:58"),
        ("timestamp", "Timestamp", "-1234", "1777777777777"),
    ];
    let mut fields = Vec::new();

    for (kind, name, first, second) in cases {
        let definition = document.id(&["field-def", "add", kind, name]);
        assert_eq!(
            document.text(&["field-def", "read", &definition]),
            format!("{kind}\t{name}")
        );
        document.success(&["field", "set", &record, &definition, first]);
        assert_eq!(
            document.text(&["field", "read", &record, &definition]),
            first
        );
        document.success(&["field", "set", &record, &definition, second]);
        assert_eq!(
            document.text(&["field", "read", &record, &definition]),
            second
        );
        fields.push((definition, kind, second));
    }

    document.success(&["field-def", "rename", &fields[0].0, "Renamed text"]);
    assert_eq!(
        document.text(&["field-def", "read", &fields[0].0]),
        "text\tRenamed text"
    );
    assert_eq!(
        document.text(&["field-def", "list"]).lines().count(),
        cases.len()
    );

    let list = document.text(&["field", "list", &record]);
    assert_eq!(list.lines().count(), cases.len());
    for (definition, kind, value) in fields {
        let value = if kind == "text" {
            "line\\nwith\\ttab\\\\slash"
        } else {
            value
        };
        assert!(
            list.lines()
                .any(|line| line == format!("{definition}\t{kind}\t{value}"))
        );
    }
}

#[test]
fn structured_data_is_marked_then_cleaned() {
    let document = TestDocument::new();
    let collection = document.id(&["collection", "add", "Values"]);
    let record = document.id(&["record", "add", &collection]);
    let first_def = document.id(&["field-def", "add", "text", "First"]);
    let second_def = document.id(&["field-def", "add", "text", "Second"]);
    document.success(&["field", "set", &record, &first_def, "first"]);
    document.success(&["field", "set", &record, &second_def, "second"]);

    document.success(&["field", "delete", &record, &first_def]);
    assert!(
        !document
            .failure(&["field", "set", &record, &first_def, "replacement"])
            .is_empty()
    );
    assert_not_found(
        &document.failure(&["field", "read", &record, &first_def]),
        "Field",
        &format!("{record}/{first_def}"),
    );
    assert!(
        document
            .text(&["field", "list", &record, "--deleted"])
            .contains(&first_def)
    );
    assert_eq!(document.count("fields", Some(true)), 1);
    document.success(&["field", "undelete", &record, &first_def]);
    assert_eq!(
        document.text(&["field", "read", &record, &first_def]),
        "first"
    );
    document.success(&["field", "delete", &record, &first_def]);

    document.success(&["field-def", "delete", &second_def]);
    assert_not_found(
        &document.failure(&["field-def", "read", &second_def]),
        "Field definition",
        &second_def,
    );
    assert!(
        document
            .text(&["field-def", "list", "--deleted"])
            .contains(&second_def)
    );
    assert!(document.text(&["field", "list", &record]).is_empty());
    assert_eq!(document.count("field_defs", Some(true)), 1);
    assert_eq!(document.count("fields", Some(true)), 1);
    document.success(&["field-def", "undelete", &second_def]);
    assert_eq!(
        document.text(&["field", "read", &record, &second_def]),
        "second"
    );
    document.success(&["field-def", "delete", &second_def]);

    document.success(&["clean"]);
    assert_eq!(document.count("fields", None), 0);
    assert_eq!(document.count("field_defs", None), 1);
    assert_eq!(document.count("records", None), 1);
    assert_eq!(document.count("collections", None), 1);

    document.success(&["field", "set", &record, &first_def, "first"]);
    document.success(&["record", "delete", &record]);
    assert!(document.text(&["record", "list", &collection]).is_empty());
    assert!(
        document
            .text(&["record", "list", &collection, "--deleted"])
            .contains(&record)
    );
    assert_eq!(document.count("records", Some(true)), 1);
    assert_eq!(document.count("fields", Some(true)), 0);
    document.success(&["record", "undelete", &record]);
    assert_eq!(
        document.text(&["field", "read", &record, &first_def]),
        "first"
    );
    document.success(&["record", "delete", &record]);
    document.success(&["clean"]);
    assert_eq!(document.count("records", None), 0);
    assert_eq!(document.count("fields", None), 0);
    assert_eq!(document.count("collections", None), 1);

    let record = document.id(&["record", "add", &collection]);
    document.success(&["field", "set", &record, &first_def, "first"]);
    document.success(&["collection", "delete", &collection]);
    assert!(document.text(&["collection", "list"]).is_empty());
    assert!(
        document
            .text(&["collection", "list", "--deleted"])
            .contains(&collection)
    );
    assert_not_found(
        &document.failure(&["record", "read", &record]),
        "Record",
        &record,
    );
    assert_eq!(document.count("collections", Some(true)), 1);
    assert_eq!(document.count("records", Some(true)), 0);
    assert_eq!(document.count("fields", Some(true)), 0);
    document.success(&["collection", "undelete", &collection]);
    assert_eq!(
        document.text(&["field", "read", &record, &first_def]),
        "first"
    );
    document.success(&["collection", "delete", &collection]);

    document.success(&["clean"]);
    assert_eq!(document.count("collections", None), 0);
    assert_eq!(document.count("records", None), 0);
    assert_eq!(document.count("fields", None), 0);
}

#[test]
fn structured_failures_do_not_change_the_document() {
    let document = TestDocument::new();
    let collection = document.id(&["collection", "add", "Values"]);
    let record = document.id(&["record", "add", &collection]);
    let missing = uuid::Uuid::now_v7().to_string();

    assert!(
        document
            .failure(&["collection", "read", "not-a-uuid"])
            .contains("invalid value")
    );
    assert_not_found(
        &document.failure(&["record", "add", &missing]),
        "Collection",
        &missing,
    );
    assert_not_found(
        &document.failure(&["field", "list", &missing]),
        "Record",
        &missing,
    );
    assert_not_found(
        &document.failure(&["field", "set", &record, &missing, "value"]),
        "Field definition",
        &missing,
    );

    let cases = [
        ("boolean", "true", "TRUE"),
        ("integer", "-42", "1.2"),
        ("date", "2026-08-21", "2026-02-30"),
        ("time", "01:02:03", "24:00:00"),
        ("timestamp", "-1234", "999999999999999999999"),
    ];
    for (kind, valid, invalid) in cases {
        let definition = document.id(&["field-def", "add", kind, kind]);
        document.success(&["field", "set", &record, &definition, valid]);
        let error = document.failure(&["field", "set", &record, &definition, invalid]);
        assert!(error.contains(&format!("expected {kind}")));
        assert_eq!(
            document.text(&["field", "read", &record, &definition]),
            valid
        );
    }

    assert_eq!(
        document.text(&["field", "list", &record]).lines().count(),
        5
    );
    assert_eq!(
        document
            .text(&["record", "list", &collection])
            .lines()
            .count(),
        1
    );
}
