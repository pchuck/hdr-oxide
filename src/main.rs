use clap::Parser;
use hdr_oxide::cli::{Cli, Commands};
use hdr_oxide::commands::{create_hdr, info_hdr};
use hdr_oxide::gui;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(args) => {
            if let Err(e) = create_hdr(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Info(args) => {
            if let Err(e) = info_hdr(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Gui => {
            if let Err(e) = gui::run_gui() {
                eprintln!("Error running GUI: {}", e);
                std::process::exit(1);
            }
        }
    }
}
