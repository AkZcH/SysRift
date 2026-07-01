mod error;
mod record;
mod replay;
mod syscall;
mod trace_log;
mod tracer;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sysrift", about = "Syscall recorder and replayer")]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
    /// Record syscalls made by a program
    Record {
        /// The program to run
        program: String,
        /// Arguments to pass to the program
        args: Vec<String>,
    },
    /// Replay a recorded trace
    Replay {
        /// The program to run
        program: String,
        /// Arguments to pass to the program
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.mode {
        Mode::Record { program, args } => record::run(&program, &args),
        Mode::Replay { program, args } => replay::run(&program, &args),
    };

    if let Err(e) = result {
        eprintln!("[sysrift] error: {}", e);
        std::process::exit(1);
    }
}