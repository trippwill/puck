use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;

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
}

fn main() {
    let args = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    match args.command {
        Commands::Document(document_args) => match document_args.command {
            DocumentCommands::New { file } => {
                tracing::info!("Creating new document: {:?}", file);
                // Implement the logic for creating a new document here
            }
        },
    }
}
