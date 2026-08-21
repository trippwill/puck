// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
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
    /// List records in a collection.
    List { collection: CollectionId },
    /// Read a record.
    Read { record: RecordId },
}

#[derive(Debug, Subcommand)]
enum FieldDefCommands {
    /// Add a field definition.
    Add { kind: String, name: String },
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

#[derive(Debug, Subcommand)]
enum FieldCommands {
    /// Set a field value.
    Set {
        record: RecordId,
        definition: FieldDefId,
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

    #[error("{0} commands are not implemented yet")]
    NotImplemented(&'static str),
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
            tracing::error!("{error}");
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
                Commands::Collection { command } => run_collection(&document, command),
                Commands::Record { command } => run_record(&document, command),
                Commands::FieldDef { command } => run_field_def(&document, command),
                Commands::Field { command } => run_field(&document, command),
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

fn run_collection(_document: &Document, _command: CollectionCommands) -> Result<(), CliError> {
    Err(CliError::NotImplemented("collection"))
}

fn run_record(_document: &Document, _command: RecordCommands) -> Result<(), CliError> {
    Err(CliError::NotImplemented("record"))
}

fn run_field_def(_document: &Document, _command: FieldDefCommands) -> Result<(), CliError> {
    Err(CliError::NotImplemented("field definition"))
}

fn run_field(_document: &Document, _command: FieldCommands) -> Result<(), CliError> {
    Err(CliError::NotImplemented("field"))
}

fn print_summaries(notes: Vec<NoteSummary>) {
    for note in notes {
        println!(
            "{}\t{}\t{}\t{}",
            note.id, note.revision, note.updated_at, note.preview
        );
    }
}
