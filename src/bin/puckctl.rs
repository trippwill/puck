use std::path::PathBuf;

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
    Document(DocumentArgs),
}

#[derive(Debug, Args)]
struct DocumentArgs {
    #[command(subcommand)]
    command: DocumentCommands,
}

#[derive(Debug, Subcommand)]
enum DocumentCommands {
    New { file: PathBuf },
    Open { file: PathBuf },
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    match args.command {
        Commands::Document(document_args) => match document_args.command {
            DocumentCommands::New { file } => {
                tracing::info!("Creating new document: {:?}", file);
                match Document::create(&file).await {
                    Ok(_) => {
                        tracing::info!("Document created successfully.");
                        println!("Document created successfully at: {:?}", file);
                    }
                    Err(e) => {
                        tracing::error!("Failed to create document: {}", e);
                    }
                }
            }
            DocumentCommands::Open { file } => {
                tracing::info!("Opening document: {:?}", file);
                match Document::open(&file).await {
                    Ok(_) => {
                        tracing::info!("Document opened successfully.");
                        println!("Document opened successfully at: {:?}", file);
                    }
                    Err(e) => {
                        tracing::error!("Failed to open document: {}", e);
                    }
                }
            }
        },
    }
}
