use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use clap::Parser;
use clap::Subcommand;

use puck::document::Document;

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
    // Create a new Puck document.
    New { file: PathBuf },
    // Validate an existing Puck document.
    Check { file: PathBuf },
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    match args.command {
        Commands::Document(document_args) => match document_args.command {
            DocumentCommands::New { file } => {
                tracing::info!("Creating new document: {:?}", file);
                match Document::create(&file).await {
                    Ok(d) => {
                        tracing::info!("Document created successfully.");
                        println!("Document created successfully at: {:?}", file.display());
                        tracing::trace!("Document path: {:?}", d.path());
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        tracing::error!("Failed to create document: {}", e);
                        ExitCode::FAILURE
                    }
                }
            }
            DocumentCommands::Check { file } => {
                tracing::info!("Opening document: {:?}", file);
                match Document::open(&file).await {
                    Ok(d) => {
                        tracing::info!("Document opened successfully.");
                        println!("Document opened successfully at: {:?}", file.display());
                        tracing::trace!("Document path: {:?}", d.path());
                        d.close();
                        tracing::debug!("Document closed.");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        tracing::error!("Failed to open document: {}", e);
                        ExitCode::FAILURE
                    }
                }
            }
        },
    }
}
