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

pub(super) const BASELINE_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 0);
const STRUCTURED_DATA_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 1);
const STRUCTURED_DELETION_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 2);
pub(super) const CURRENT_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 3);
pub(super) const MINIMUM_COMPATIBLE_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 0);

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
    Migration::new(BASELINE_VERSION, include_str!("migrations/0.0.0.sql")),
    Migration::new(
        STRUCTURED_DATA_VERSION,
        include_str!("migrations/0.0.1.sql"),
    ),
    Migration::new(
        STRUCTURED_DELETION_VERSION,
        include_str!("migrations/0.0.2.sql"),
    ),
    Migration::new(CURRENT_VERSION, include_str!("migrations/0.0.3.sql")),
];

#[derive(Debug)]
pub(super) enum MigrationError {
    InvalidRegistry,
    Sqlite(SqlError),
    UnregisteredVersion(SchemaVersion),
}

impl From<SqlError> for MigrationError {
    fn from(error: SqlError) -> Self {
        Self::Sqlite(error)
    }
}

pub(super) fn initialize(tx: &Transaction<'_>) -> Result<SchemaVersion, MigrationError> {
    initialize_with(tx, production_migrations()?)
}

pub(super) fn migrate(
    conn: &mut SyncConnection,
    from: SchemaVersion,
) -> Result<SchemaVersion, MigrationError> {
    migrate_with(conn, from, production_migrations()?)
}

fn migrate_with(
    conn: &mut SyncConnection,
    from: SchemaVersion,
    migrations: &[Migration],
) -> Result<SchemaVersion, MigrationError> {
    validate_registry(migrations)?;

    let current = migrations
        .last()
        .expect("validated migration registry is not empty")
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
    validate_registry(migrations)?;
    apply(tx, migrations)?;

    let current = migrations
        .last()
        .expect("validated migration registry is not empty")
        .version;
    tx.pragma_update(None, "user_version", i32::from(current))?;
    Ok(current)
}

fn apply(tx: &Transaction<'_>, migrations: &[Migration]) -> SqlResult<()> {
    for migration in migrations {
        tx.execute_batch(migration.sql)?;
    }
    Ok(())
}

fn production_migrations() -> Result<&'static [Migration], MigrationError> {
    validate_registry(MIGRATIONS)?;

    let first = MIGRATIONS
        .first()
        .expect("validated migration registry is not empty");
    let last = MIGRATIONS
        .last()
        .expect("validated migration registry is not empty");

    if first.version != BASELINE_VERSION
        || last.version != CURRENT_VERSION
        || !MIGRATIONS
            .iter()
            .any(|migration| migration.version == MINIMUM_COMPATIBLE_VERSION)
        || MIGRATIONS
            .iter()
            .any(|migration| !migration_sql_is_safe(migration.sql))
    {
        return Err(MigrationError::InvalidRegistry);
    }

    Ok(MIGRATIONS)
}

fn validate_registry(migrations: &[Migration]) -> Result<(), MigrationError> {
    if migrations.is_empty()
        || migrations
            .windows(2)
            .any(|pair| pair[0].version >= pair[1].version)
    {
        return Err(MigrationError::InvalidRegistry);
    }
    Ok(())
}

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
        SchemaVersion::from_i32(raw)
    }

    fn normalize_sql(sql: &str) -> String {
        sql.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_end_matches(';')
            .to_owned()
    }

    #[test]
    fn baseline_creates_current_schema() {
        let mut conn = SyncConnection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        assert_eq!(initialize(&tx).unwrap(), CURRENT_VERSION);
        tx.commit().unwrap();

        let schema = conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type = 'table' AND name = 'notes'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        let baseline = MIGRATIONS[0]
            .sql
            .split_once("CREATE TABLE")
            .map(|(_, sql)| format!("CREATE TABLE{sql}"))
            .unwrap();
        let columns = baseline
            .trim()
            .strip_suffix(") STRICT;")
            .expect("baseline notes table is strict");
        let expected = format!(
            "{columns}, deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1))) STRICT"
        );

        assert_eq!(normalize_sql(&schema), normalize_sql(&expected));
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
        let migrations = production_migrations().unwrap();

        assert_eq!(migrations.first().unwrap().version, BASELINE_VERSION);
        assert_eq!(migrations.last().unwrap().version, CURRENT_VERSION);
        assert!(BASELINE_VERSION <= MINIMUM_COMPATIBLE_VERSION);
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
