mod ast;
mod backend;
mod cli;
mod diagnostics;
mod formatter;
mod hal;
mod interpreter;
mod lexer;
mod parser;
mod semantic;
mod utils;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
