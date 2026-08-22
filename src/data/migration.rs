// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Embedded `SQLite` schema migrations.
//!
//! New documents replay the full ordered registry, while existing documents
//! apply only entries newer than their `PRAGMA user_version`. Schema changes
//! and the resulting version are committed together, so a failed chain leaves
//! the document unchanged. Applied migration files are immutable.

use tokio_rusqlite::Transaction;
use tokio_rusqlite::rusqlite::{
    Connection as SyncConnection,
    Error as SqlError,
    Result as SqlResult,
};

use super::version::SchemaVersion;

pub(super) const BASELINE_VERSION: SchemaVersion = SchemaVersion::new(0, 1, 0);
pub(super) const CURRENT_VERSION: SchemaVersion = SchemaVersion::new(0, 1, 2);
pub(super) const MINIMUM_COMPATIBLE_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 4);

#[derive(Clone, Copy)]
struct Migration {
    version: SchemaVersion,
    sql: &'static str,
}

impl Migration {
    const fn new(version: SchemaVersion, sql: &'static str) -> Self {
        Self { version, sql }
    }
}

const MIGRATIONS: &[Migration] = &[
    Migration::new(MINIMUM_COMPATIBLE_VERSION, ""),
    Migration::new(BASELINE_VERSION, include_str!("migrations/0.1.0.sql")),
    Migration::new(
        SchemaVersion::new(0, 1, 1),
        include_str!("migrations/0.1.1.sql"),
    ),
    Migration::new(CURRENT_VERSION, include_str!("migrations/0.1.2.sql")),
];

#[derive(Debug)]
pub(super) enum MigrationError {
    Sqlite(SqlError),
    UnregisteredVersion(SchemaVersion),
}

impl From<SqlError> for MigrationError {
    fn from(error: SqlError) -> Self {
        Self::Sqlite(error)
    }
}

pub(super) fn initialize(tx: &Transaction<'_>) -> Result<SchemaVersion, MigrationError> {
    initialize_with(tx, MIGRATIONS)
}

pub(super) fn migrate(
    conn: &mut SyncConnection,
    from: SchemaVersion,
) -> Result<SchemaVersion, MigrationError> {
    migrate_with(conn, from, MIGRATIONS)
}

fn migrate_with(
    conn: &mut SyncConnection,
    from: SchemaVersion,
    migrations: &[Migration],
) -> Result<SchemaVersion, MigrationError> {
    let current = migrations
        .last()
        .expect("migration registry is not empty")
        .version;
    let index = migrations
        .iter()
        .position(|migration| migration.version == from)
        .ok_or(MigrationError::UnregisteredVersion(from))?;
    let pending = &migrations[index + 1..];

    if pending.is_empty() {
        return Ok(from);
    }

    let tx = conn.transaction()?;
    apply(&tx, pending)?;
    tx.pragma_update(None, "user_version", i32::from(current))?;
    tx.commit()?;
    Ok(current)
}

fn initialize_with(
    tx: &Transaction<'_>,
    migrations: &[Migration],
) -> Result<SchemaVersion, MigrationError> {
    apply(tx, migrations)?;

    let current = migrations
        .last()
        .expect("migration registry is not empty")
        .version;
    tx.pragma_update(None, "user_version", i32::from(current))?;
    Ok(current)
}

fn apply(tx: &Transaction<'_>, migrations: &[Migration]) -> SqlResult<()> {
    for migration in migrations {
        tracing::debug!(version = %migration.version, "Applying schema migration");
        tx.execute_batch(migration.sql)?;
    }
    Ok(())
}

#[cfg(test)]
fn migration_sql_is_safe(sql: &str) -> bool {
    const FORBIDDEN: &[&str] = &[
        "ATTACH",
        "BEGIN",
        "COMMIT",
        "DETACH",
        "PRAGMA",
        "RELEASE",
        "ROLLBACK",
        "SAVEPOINT",
        "VACUUM",
    ];

    !sql.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| {
            FORBIDDEN
                .iter()
                .any(|forbidden| token.eq_ignore_ascii_case(forbidden))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const V0: SchemaVersion = SchemaVersion::new(0, 0, 0);
    const V1: SchemaVersion = SchemaVersion::new(0, 0, 1);
    const V2: SchemaVersion = SchemaVersion::new(0, 0, 2);

    const TEST_MIGRATIONS: &[Migration] = &[
        Migration::new(V0, "CREATE TABLE steps (value INTEGER NOT NULL) STRICT;"),
        Migration::new(V1, "INSERT INTO steps VALUES (1);"),
        Migration::new(V2, "INSERT INTO steps VALUES (2);"),
    ];

    fn user_version(conn: &SyncConnection) -> SchemaVersion {
        let raw = conn
            .pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
            .unwrap();
        SchemaVersion::from(raw)
    }

    #[test]
    fn baseline_creates_current_schema() {
        let mut conn = SyncConnection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        assert_eq!(initialize(&tx).unwrap(), CURRENT_VERSION);
        tx.commit().unwrap();

        let objects = conn
            .prepare(
                r"
                SELECT name
                FROM sqlite_schema
                WHERE type IN ('table', 'index') AND name NOT LIKE 'sqlite_%'
                ORDER BY name
                ",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<SqlResult<Vec<_>>>()
            .unwrap();
        assert_eq!(
            objects,
            [
                "collections",
                "field_defs",
                "fields",
                "fields_by_definition",
                "notes",
                "records",
                "records_by_collection",
                "records_by_source_note",
            ]
        );
        assert_eq!(user_version(&conn), CURRENT_VERSION);
    }

    #[test]
    fn minimum_compatible_document_migrates_without_losing_data() {
        let mut conn = SyncConnection::open_in_memory().unwrap();
        let id = vec![0_u8; 16];
        conn.execute_batch(
            r"
            CREATE TABLE notes (
                id BLOB PRIMARY KEY NOT NULL,
                body TEXT NOT NULL,
                revision INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL CHECK (updated_at >= created_at),
                archived INTEGER NOT NULL DEFAULT 0,
                deleted INTEGER NOT NULL DEFAULT 0
            ) STRICT;
            ",
        )
        .unwrap();
        conn.execute(
            r"
            INSERT INTO notes (
                id, body, revision, created_at, updated_at, archived, deleted
            )
            VALUES (
                ?1,
                'Keep me',
                1,
                '2026-08-18 14:15:40.601255975+00:00',
                '2026-08-18 14:15:41.701255975+00:00',
                0,
                0
            )
            ",
            [&id],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", i32::from(MINIMUM_COMPATIBLE_VERSION))
            .unwrap();

        assert_eq!(
            migrate(&mut conn, MINIMUM_COMPATIBLE_VERSION).unwrap(),
            CURRENT_VERSION
        );
        assert_eq!(
            conn.query_row(
                r"
                SELECT
                    body,
                    typeof(created_at),
                    typeof(updated_at),
                    updated_at > created_at
                FROM notes
                WHERE id = ?1
                ",
                [&id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .unwrap(),
            (
                String::from("Keep me"),
                String::from("integer"),
                String::from("integer"),
                true,
            )
        );
        assert_eq!(user_version(&conn), CURRENT_VERSION);
    }

    #[test]
    fn migrations_apply_in_order() {
        let mut conn = SyncConnection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        assert_eq!(initialize_with(&tx, TEST_MIGRATIONS).unwrap(), V2);
        tx.commit().unwrap();

        let values = conn
            .prepare("SELECT value FROM steps ORDER BY rowid")
            .unwrap()
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<SqlResult<Vec<_>>>()
            .unwrap();
        assert_eq!(values, [1, 2]);
        assert_eq!(user_version(&conn), V2);
    }

    #[test]
    fn failed_migration_rolls_back_chain_and_version() {
        const FAILING_MIGRATIONS: &[Migration] = &[
            Migration::new(V0, "CREATE TABLE steps (value INTEGER NOT NULL) STRICT;"),
            Migration::new(V1, "INSERT INTO steps VALUES (1);"),
            Migration::new(V2, "INSERT INTO missing_table VALUES (2);"),
        ];

        let mut conn = SyncConnection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        initialize_with(&tx, &FAILING_MIGRATIONS[..1]).unwrap();
        tx.commit().unwrap();

        assert!(matches!(
            migrate_with(&mut conn, V0, FAILING_MIGRATIONS),
            Err(MigrationError::Sqlite(_))
        ));
        assert_eq!(
            conn.query_row("SELECT count(*) FROM steps", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(user_version(&conn), V0);
    }

    #[test]
    fn unregistered_version_does_not_modify_database() {
        let mut conn = SyncConnection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        initialize_with(&tx, &TEST_MIGRATIONS[..1]).unwrap();
        tx.commit().unwrap();
        let unknown = SchemaVersion::new(0, 0, 99);

        assert!(matches!(
            migrate_with(&mut conn, unknown, TEST_MIGRATIONS),
            Err(MigrationError::UnregisteredVersion(version)) if version == unknown
        ));
        assert_eq!(
            conn.query_row("SELECT count(*) FROM steps", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(user_version(&conn), V0);
    }

    #[test]
    fn production_registry_is_valid_and_transaction_safe() {
        let migrations = MIGRATIONS;

        assert!(!migrations.is_empty());
        assert!(
            !migrations
                .windows(2)
                .any(|pair| pair[0].version >= pair[1].version)
        );
        assert_eq!(
            migrations.first().unwrap().version,
            MINIMUM_COMPATIBLE_VERSION
        );
        assert_eq!(migrations.last().unwrap().version, CURRENT_VERSION);
        assert!(MINIMUM_COMPATIBLE_VERSION <= BASELINE_VERSION);
        assert!(BASELINE_VERSION <= CURRENT_VERSION);
        assert!(MINIMUM_COMPATIBLE_VERSION <= CURRENT_VERSION);
        assert!(
            migrations
                .iter()
                .all(|migration| migration_sql_is_safe(migration.sql))
        );
        assert!(!migration_sql_is_safe("BEGIN; SELECT 1; COMMIT;"));
        assert!(!migration_sql_is_safe("PRAGMA journal_mode = WAL;"));
    }
}
