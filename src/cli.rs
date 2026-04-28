use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::diagnostics::Diagnostic;

#[derive(Debug, Parser)]
#[command(name = "light")]
#[command(about = "Light language toolchain (MVP)")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Run {
        file: PathBuf,
    },
    Ast {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Check {
        file: PathBuf,
    },
    Fmt {
        file: PathBuf,
    },
    Build {
        file: PathBuf,
        #[arg(long)]
        target: Option<String>,
        #[arg(long = "no-std", default_value_t = false)]
        no_std: bool,
    },
    Firmware {
        #[command(subcommand)]
        command: FirmwareCommands,
    },
    Flash {
        image: PathBuf,
        #[arg(long)]
        target: String,
        #[arg(long)]
        interface: Option<String>,
        #[arg(long)]
        port: Option<String>,
    },
    Targets {
        #[command(subcommand)]
        command: TargetCommands,
    },
}

#[derive(Debug, Subcommand)]
enum FirmwareCommands {
    Build {
        file: PathBuf,
        #[arg(long)]
        target: String,
    },
}

#[derive(Debug, Subcommand)]
enum TargetCommands {
    List,
}

pub fn run() -> Result<(), Diagnostic> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => {
            let program = crate::utils::load_program(&file)?;
            crate::semantic::analyze(&program)?;
            crate::interpreter::run(&program)
        }
        Commands::Ast { file, json } => {
            let program = crate::utils::load_program(&file)?;
            if json {
                let json = serde_json::to_string_pretty(&program).expect("serialize ast");
                println!("{json}");
            } else {
                println!("{:#?}", program);
            }
            Ok(())
        }
        Commands::Check { file } => {
            let program = crate::utils::load_program(&file)?;
            crate::semantic::analyze(&program)?;
            println!("ok");
            Ok(())
        }
        Commands::Fmt { file } => {
            let source = fs::read_to_string(&file).map_err(|e| {
                Diagnostic::new(
                    crate::diagnostics::DiagnosticCode::RuntimeError,
                    e.to_string(),
                    crate::diagnostics::Span::new(0, 0),
                )
            })?;
            let formatted = crate::formatter::format_source(&source)?;
            fs::write(&file, formatted).map_err(|e| {
                Diagnostic::new(
                    crate::diagnostics::DiagnosticCode::RuntimeError,
                    e.to_string(),
                    crate::diagnostics::Span::new(0, 0),
                )
            })?;
            Ok(())
        }
        Commands::Build {
            file,
            target,
            no_std,
        } => {
            let _ = crate::utils::load_program(&file)?;
            crate::backend::build(target.unwrap_or_else(|| "x86_64-linux".to_string()), no_std)
        }
        Commands::Firmware { command } => match command {
            FirmwareCommands::Build { file, target } => {
                let _ = crate::utils::load_program(&file)?;
                crate::hal::build_firmware(&target)
            }
        },
        Commands::Flash {
            image,
            target,
            interface,
            port,
        } => crate::hal::flash_image(&image, &target, interface.as_deref(), port.as_deref()),
        Commands::Targets { command } => match command {
            TargetCommands::List => {
                for target in crate::backend::supported_targets() {
                    println!("{target}");
                }
                Ok(())
            }
        },
    }
}
