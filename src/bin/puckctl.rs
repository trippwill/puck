use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use puck::core::{NoteId, PileNote};
use puck::data::Document;
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Debug, Subcommand)]
enum Commands {
    // Manage Puck documents.
    #[command(visible_alias = "doc")]
    Document(DocumentArgs),
}

#[derive(Debug, Args)]
struct DocumentArgs {
    #[command(subcommand)]
    command: DocumentCommands,
}

#[derive(Debug, Subcommand)]
enum DocumentCommands {
    /// Create a new Puck document.
    New { file: PathBuf },
    /// Validate an existing Puck document.
    Check { file: PathBuf },
    /// Manage notes.
    Note(NoteArgs),
}

#[derive(Debug, Args)]
struct NoteArgs {
    #[command(subcommand)]
    command: NoteCommands,
}

#[derive(Debug, Subcommand)]
enum NoteCommands {
    /// Add a pile note.
    Add { file: PathBuf, body: String },
    /// List pile notes.
    List { file: PathBuf },
    /// Read a pile note.
    Read { file: PathBuf, note: NoteId },
}

#[derive(Debug, Error)]
enum CliError {
    #[error(transparent)]
    Document(#[from] puck::data::DocumentError),

    #[error("Note {0} not found")]
    NoteNotFound(NoteId),
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    match run(args.command).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(command: Commands) -> Result<(), CliError> {
    match command {
        Commands::Document(document_args) => match document_args.command {
            DocumentCommands::New { file } => {
                tracing::info!("Creating new document: {:?}", file);
                let document = Document::create(&file).await?;
                tracing::info!("Document created successfully.");
                println!("Document created successfully at: {:?}", file.display());
                tracing::trace!(
                    "Document path: {:?}, version: {:?}",
                    document.path(),
                    document.version()
                );
            }
            DocumentCommands::Check { file } => {
                tracing::info!("Opening document: {:?}", file);
                let document = Document::open(&file).await?;
                tracing::info!("Document opened successfully.");
                println!("Document opened successfully at: {:?}", file.display());
                tracing::trace!(
                    "Document path: {:?}, version: {:?}",
                    document.path(),
                    document.version()
                );
            }
            DocumentCommands::Note(args) => match args.command {
                NoteCommands::Add { file, body } => {
                    let document = Document::open(file).await?;
                    let note = PileNote::create(body);
                    let id = note.id();
                    document.add_note(note).await?;
                    println!("{id}");
                }
                NoteCommands::List { file } => {
                    let document = Document::open(file).await?;
                    for note in document.note_summaries().await? {
                        println!(
                            "{}\t{}\t{}\t{}",
                            note.id, note.revision, note.updated_at, note.preview
                        );
                    }
                }
                NoteCommands::Read { file, note } => {
                    let document = Document::open(file).await?;
                    let note = document
                        .note(note)
                        .await?
                        .ok_or(CliError::NoteNotFound(note))?;
                    print!("{}", note.body());
                }
            },
        },
    }

    Ok(())
}
