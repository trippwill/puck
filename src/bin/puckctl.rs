// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use puck::core::prelude::*;
use puck::data::prelude::*;
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// The Puck document to use.
    file: PathBuf,

    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new Puck document.
    New,
    /// Validate an existing Puck document.
    Check,
    /// Permanently remove structured data marked for deletion.
    Clean,
    /// Manage notes.
    Note {
        #[command(subcommand)]
        command: NoteCommands,
    },
    /// Manage collections.
    Collection {
        #[command(subcommand)]
        command: CollectionCommands,
    },
    /// Manage records.
    Record {
        #[command(subcommand)]
        command: RecordCommands,
    },
    /// Manage field definitions.
    FieldDef {
        #[command(subcommand)]
        command: FieldDefCommands,
    },
    /// Manage field values.
    Field {
        #[command(subcommand)]
        command: FieldCommands,
    },
}

#[derive(Debug, Subcommand)]
enum NoteCommands {
    /// Add a pile note.
    Add { body: String },
    /// Archive a pile note.
    Archive { note: NoteId },
    /// Edit a pile note.
    Edit { note: NoteId, body: String },
    /// Mark an archived note for deletion.
    Delete { note: NoteId },
    /// List pile notes.
    List {
        /// List archived notes instead.
        #[arg(long)]
        archived: bool,
    },
    /// Read a pile note.
    Read {
        /// Read an archived note instead.
        #[arg(long)]
        archived: bool,
        note: NoteId,
    },
    /// Search active note bodies.
    Search { text: String },
    /// Return an archived note to the active pile.
    Unarchive { note: NoteId },
}

#[derive(Debug, Subcommand)]
enum CollectionCommands {
    /// Add a collection.
    Add { name: String },
    /// Mark a collection and its contents for deletion.
    Delete { collection: CollectionId },
    /// List collections.
    List,
    /// Read a collection.
    Read { collection: CollectionId },
    /// Rename a collection.
    Rename {
        collection: CollectionId,
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum RecordCommands {
    /// Add a record to a collection.
    Add { collection: CollectionId },
    /// Mark a record and its fields for deletion.
    Delete { record: RecordId },
    /// List records in a collection.
    List { collection: CollectionId },
    /// Read a record.
    Read { record: RecordId },
}

#[derive(Debug, Subcommand)]
enum FieldDefCommands {
    /// Add a field definition.
    Add { kind: FieldKind, name: String },
    /// Mark a field definition and its values for deletion.
    Delete { definition: FieldDefId },
    /// List field definitions.
    List,
    /// Read a field definition.
    Read { definition: FieldDefId },
    /// Rename a field definition.
    Rename {
        definition: FieldDefId,
        name: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FieldKind {
    Text,
    Boolean,
    Integer,
    Date,
    Time,
    Timestamp,
}

#[derive(Debug, Subcommand)]
enum FieldCommands {
    /// Mark a field value for deletion.
    Delete {
        record: RecordId,
        definition: FieldDefId,
    },
    /// Set a field value.
    Set {
        record: RecordId,
        definition: FieldDefId,
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    /// List fields on a record.
    List { record: RecordId },
    /// Read a field value.
    Read {
        record: RecordId,
        definition: FieldDefId,
    },
}

#[derive(Debug, Error)]
enum CliError {
    #[error(transparent)]
    Document(#[from] puck::data::DocumentError),

    #[error(transparent)]
    Note(#[from] NoteError),

    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },

    #[error("invalid value {value:?}: expected {expected}")]
    InvalidValue {
        value: String,
        expected: &'static str,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    match run(args.file, args.command).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(file: PathBuf, command: Commands) -> Result<(), CliError> {
    match command {
        Commands::New => create_document(file).await,
        Commands::Check => check_document(file).await,
        command => {
            tracing::info!("Opening document: {:?}", file);
            let document = Document::open(file).await?;
            match command {
                Commands::Note { command } => run_note(&document, command).await,
                Commands::Collection { command } => run_collection(&document, command).await,
                Commands::Record { command } => run_record(&document, command).await,
                Commands::FieldDef { command } => run_field_def(&document, command).await,
                Commands::Field { command } => run_field(&document, command).await,
                Commands::Clean => {
                    document.execute(vec![Command::Clean]).await?;
                    Ok(())
                }
                Commands::New | Commands::Check => unreachable!(),
            }
        }
    }
}

async fn create_document(file: PathBuf) -> Result<(), CliError> {
    tracing::info!("Creating new document: {:?}", file);
    let document = Document::create(&file).await?;
    tracing::info!("Document created successfully.");
    println!("Document created successfully at: {:?}", file.display());
    tracing::trace!(
        "Document path: {:?}, version: {:?}",
        document.path(),
        document.version()
    );
    Ok(())
}

async fn check_document(file: PathBuf) -> Result<(), CliError> {
    tracing::info!("Opening document: {:?}", file);
    let document = Document::open(&file).await?;
    tracing::info!("Document opened successfully.");
    println!("Document opened successfully at: {:?}", file.display());
    tracing::trace!(
        "Document path: {:?}, version: {:?}",
        document.path(),
        document.version()
    );
    Ok(())
}

async fn run_note(document: &Document, command: NoteCommands) -> Result<(), CliError> {
    match command {
        NoteCommands::Add { body } => {
            let note = PileNote::create(body);
            let id = note.id();
            document.execute(vec![Command::AddNote(note)]).await?;
            println!("{id}");
        }
        NoteCommands::Archive { note } => {
            let note = document
                .query(NoteById(note))
                .await?
                .ok_or_else(|| not_found("Note", note.to_string()))?
                .archive();
            document.execute(vec![Command::ArchiveNote(note)]).await?;
        }
        NoteCommands::Edit { note, body } => {
            let note = document
                .query(NoteById(note))
                .await?
                .ok_or_else(|| not_found("Note", note.to_string()))?
                .edit(body)?;
            document.execute(vec![Command::EditNote(note)]).await?;
        }
        NoteCommands::Delete { note } => {
            document
                .query(ArchivedNoteById(note))
                .await?
                .ok_or_else(|| not_found("Archived note", note.to_string()))?;
            document.execute(vec![Command::DeleteNote(note)]).await?;
        }
        NoteCommands::List { archived } => {
            let notes = if archived {
                document.query(ArchivedNoteSummaries).await?
            } else {
                document.query(NoteSummaries).await?
            };
            print_summaries(notes);
        }
        NoteCommands::Read { archived, note } => {
            if archived {
                let note = document
                    .query(ArchivedNoteById(note))
                    .await?
                    .ok_or_else(|| not_found("Note", note.to_string()))?;
                print!("{}", note.body());
            } else {
                let note = document
                    .query(NoteById(note))
                    .await?
                    .ok_or_else(|| not_found("Note", note.to_string()))?;
                print!("{}", note.body());
            }
        }
        NoteCommands::Search { text } => {
            print_summaries(document.query(NoteSearch(text)).await?);
        }
        NoteCommands::Unarchive { note } => {
            let note = document
                .query(ArchivedNoteById(note))
                .await?
                .ok_or_else(|| not_found("Note", note.to_string()))?
                .unarchive();
            document.execute(vec![Command::UnarchiveNote(note)]).await?;
        }
    }
    Ok(())
}

fn not_found(kind: &'static str, id: String) -> CliError {
    CliError::NotFound { kind, id }
}

async fn run_collection(document: &Document, command: CollectionCommands) -> Result<(), CliError> {
    match command {
        CollectionCommands::Add { name } => {
            let collection = Collection::new(&name);
            let id = collection.id();
            document
                .execute(vec![Command::UpsertCollection(collection)])
                .await?;
            println!("{id}");
        }
        CollectionCommands::Delete { collection } => {
            document
                .query(CollectionById(collection))
                .await?
                .ok_or_else(|| not_found("Collection", collection.to_string()))?;
            document
                .execute(vec![Command::DeleteCollection(collection)])
                .await?;
        }
        CollectionCommands::List => {
            for collection in document.query(Collections).await? {
                println!("{}\t{}", collection.id(), collection.name());
            }
        }
        CollectionCommands::Read { collection } => {
            let collection = document
                .query(CollectionById(collection))
                .await?
                .ok_or_else(|| not_found("Collection", collection.to_string()))?;
            print!("{}", collection.name());
        }
        CollectionCommands::Rename { collection, name } => {
            let mut collection = document
                .query(CollectionById(collection))
                .await?
                .ok_or_else(|| not_found("Collection", collection.to_string()))?;
            collection.set_name(&name);
            document
                .execute(vec![Command::UpsertCollection(collection)])
                .await?;
        }
    }
    Ok(())
}

async fn run_record(document: &Document, command: RecordCommands) -> Result<(), CliError> {
    match command {
        RecordCommands::Add { collection } => {
            let collection = document
                .query(CollectionById(collection))
                .await?
                .ok_or_else(|| not_found("Collection", collection.to_string()))?;
            let record = collection.new_record();
            let id = record.id();
            document
                .execute(vec![Command::UpsertRecord(record)])
                .await?;
            println!("{id}");
        }
        RecordCommands::Delete { record } => {
            document
                .query(RecordById(record))
                .await?
                .ok_or_else(|| not_found("Record", record.to_string()))?;
            document
                .execute(vec![Command::DeleteRecord(record)])
                .await?;
        }
        RecordCommands::List { collection } => {
            document
                .query(CollectionById(collection))
                .await?
                .ok_or_else(|| not_found("Collection", collection.to_string()))?;
            for record in document.query(RecordsByCollection(collection)).await? {
                println!("{}\t{}", record.id(), record.collection_id());
            }
        }
        RecordCommands::Read { record } => {
            let record = document
                .query(RecordById(record))
                .await?
                .ok_or_else(|| not_found("Record", record.to_string()))?;
            print!("{}\t{}", record.id(), record.collection_id());
        }
    }
    Ok(())
}

async fn run_field_def(document: &Document, command: FieldDefCommands) -> Result<(), CliError> {
    match command {
        FieldDefCommands::Add { kind, name } => {
            let field_def = match kind {
                FieldKind::Text => AnyFieldDef::Text(Text::def(&name)),
                FieldKind::Boolean => AnyFieldDef::Boolean(Boolean::def(&name)),
                FieldKind::Integer => AnyFieldDef::Integer(Integer::def(&name)),
                FieldKind::Date => AnyFieldDef::Date(Date::def(&name)),
                FieldKind::Time => AnyFieldDef::Time(Time::def(&name)),
                FieldKind::Timestamp => AnyFieldDef::Timestamp(Timestamp::def(&name)),
            };
            let id = field_def.id();
            document
                .execute(vec![Command::UpsertFieldDef(field_def)])
                .await?;
            println!("{id}");
        }
        FieldDefCommands::Delete { definition } => {
            document
                .query(FieldDefById(definition))
                .await?
                .ok_or_else(|| not_found("Field definition", definition.to_string()))?;
            document
                .execute(vec![Command::DeleteFieldDef(definition)])
                .await?;
        }
        FieldDefCommands::List => {
            for field_def in document.query(FieldDefs).await? {
                println!(
                    "{}\t{}\t{}",
                    field_def.id(),
                    field_def_kind(&field_def),
                    field_def.name()
                );
            }
        }
        FieldDefCommands::Read { definition } => {
            let field_def = document
                .query(FieldDefById(definition))
                .await?
                .ok_or_else(|| not_found("Field definition", definition.to_string()))?;
            print!("{}\t{}", field_def_kind(&field_def), field_def.name());
        }
        FieldDefCommands::Rename { definition, name } => {
            let mut field_def = document
                .query(FieldDefById(definition))
                .await?
                .ok_or_else(|| not_found("Field definition", definition.to_string()))?;
            match &mut field_def {
                AnyFieldDef::Text(def) => def.set_name(&name),
                AnyFieldDef::Boolean(def) => def.set_name(&name),
                AnyFieldDef::Integer(def) => def.set_name(&name),
                AnyFieldDef::Date(def) => def.set_name(&name),
                AnyFieldDef::Time(def) => def.set_name(&name),
                AnyFieldDef::Timestamp(def) => def.set_name(&name),
            }
            document
                .execute(vec![Command::UpsertFieldDef(field_def)])
                .await?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn run_field(document: &Document, command: FieldCommands) -> Result<(), CliError> {
    match command {
        FieldCommands::Delete { record, definition } => {
            document
                .query(RecordById(record))
                .await?
                .ok_or_else(|| not_found("Record", record.to_string()))?;
            document
                .query(FieldDefById(definition))
                .await?
                .ok_or_else(|| not_found("Field definition", definition.to_string()))?;
            document
                .query(FieldByKey((record, definition)))
                .await?
                .ok_or_else(|| not_found("Field", format!("{record}/{definition}")))?;
            document
                .execute(vec![Command::DeleteField((record, definition))])
                .await?;
        }
        FieldCommands::Set {
            record,
            definition,
            value,
        } => {
            let stored_record = document
                .query(RecordById(record))
                .await?
                .ok_or_else(|| not_found("Record", record.to_string()))?;
            let field_def = document
                .query(FieldDefById(definition))
                .await?
                .ok_or_else(|| not_found("Field definition", definition.to_string()))?;
            let field = match field_def {
                AnyFieldDef::Text(def) => AnyField::Text(stored_record.new_field(&def, value)),
                AnyFieldDef::Boolean(def) => {
                    let parsed = match value.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(invalid_value(&value, "boolean (true or false)")),
                    };
                    AnyField::Boolean(stored_record.new_field(&def, parsed))
                }
                AnyFieldDef::Integer(def) => {
                    let parsed = value
                        .parse()
                        .map_err(|_| invalid_value(&value, "integer"))?;
                    AnyField::Integer(stored_record.new_field(&def, parsed))
                }
                AnyFieldDef::Date(def) => {
                    let format =
                        time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
                            .expect("valid date format");
                    let parsed = time::Date::parse(&value, &format)
                        .map_err(|_| invalid_value(&value, "date (YYYY-MM-DD)"))?;
                    AnyField::Date(stored_record.new_field(&def, parsed))
                }
                AnyFieldDef::Time(def) => {
                    let format =
                        time::format_description::parse_borrowed::<2>("[hour]:[minute]:[second]")
                            .expect("valid time format");
                    let parsed = time::Time::parse(&value, &format)
                        .map_err(|_| invalid_value(&value, "time (HH:MM:SS)"))?;
                    AnyField::Time(stored_record.new_field(&def, parsed))
                }
                AnyFieldDef::Timestamp(def) => {
                    let milliseconds = value
                        .parse()
                        .map_err(|_| invalid_value(&value, "timestamp (Unix milliseconds)"))?;
                    let parsed = time::Timestamp::from_milliseconds(milliseconds)
                        .map_err(|_| invalid_value(&value, "timestamp (Unix milliseconds)"))?;
                    AnyField::Timestamp(stored_record.new_field(&def, parsed))
                }
            };
            document.execute(vec![Command::UpsertField(field)]).await?;
        }
        FieldCommands::List { record } => {
            document
                .query(RecordById(record))
                .await?
                .ok_or_else(|| not_found("Record", record.to_string()))?;
            for field in document.query(FieldsByRecord(record)).await? {
                println!(
                    "{}\t{}\t{}",
                    field.def_id(),
                    field_kind(&field),
                    list_field_value(&field)
                );
            }
        }
        FieldCommands::Read { record, definition } => {
            document
                .query(RecordById(record))
                .await?
                .ok_or_else(|| not_found("Record", record.to_string()))?;
            document
                .query(FieldDefById(definition))
                .await?
                .ok_or_else(|| not_found("Field definition", definition.to_string()))?;
            let field = document
                .query(FieldByKey((record, definition)))
                .await?
                .ok_or_else(|| not_found("Field", format!("{record}/{definition}")))?;
            print!("{}", field_value(&field));
        }
    }
    Ok(())
}

fn invalid_value(value: &str, expected: &'static str) -> CliError {
    CliError::InvalidValue {
        value: value.to_owned(),
        expected,
    }
}

fn field_def_kind(field_def: &AnyFieldDef) -> &'static str {
    match field_def {
        AnyFieldDef::Text(_) => "text",
        AnyFieldDef::Boolean(_) => "boolean",
        AnyFieldDef::Integer(_) => "integer",
        AnyFieldDef::Date(_) => "date",
        AnyFieldDef::Time(_) => "time",
        AnyFieldDef::Timestamp(_) => "timestamp",
    }
}

fn field_kind(field: &AnyField) -> &'static str {
    match field {
        AnyField::Text(_) => "text",
        AnyField::Boolean(_) => "boolean",
        AnyField::Integer(_) => "integer",
        AnyField::Date(_) => "date",
        AnyField::Time(_) => "time",
        AnyField::Timestamp(_) => "timestamp",
    }
}

fn field_value(field: &AnyField) -> String {
    match field {
        AnyField::Text(field) => field.val().clone(),
        AnyField::Boolean(field) => field.val().to_string(),
        AnyField::Integer(field) => field.val().to_string(),
        AnyField::Date(field) => field.val().to_string(),
        AnyField::Time(field) => format!(
            "{:02}:{:02}:{:02}",
            field.val().hour(),
            field.val().minute(),
            field.val().second()
        ),
        AnyField::Timestamp(field) => field.val().as_milliseconds().to_string(),
    }
}

fn list_field_value(field: &AnyField) -> String {
    match field {
        AnyField::Text(field) => field
            .val()
            .replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\r', "\\r")
            .replace('\n', "\\n"),
        _ => field_value(field),
    }
}

fn print_summaries(notes: Vec<NoteSummary>) {
    for note in notes {
        println!(
            "{}\t{}\t{}\t{}",
            note.id, note.revision, note.updated_at, note.preview
        );
    }
}
