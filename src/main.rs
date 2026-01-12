//! CLI entry point for mdbook-termlink preprocessor.

use std::io;
use std::process;

use anyhow::Result;
use mdbook_preprocessor::{Preprocessor, parse_input};

use mdbook_termlink::TermlinkPreprocessor;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    // Handle "supports <renderer>" check
    if args.len() >= 3 && args[1] == "supports" {
        let renderer = &args[2];
        // Only support HTML renderer
        process::exit(i32::from(renderer != "html"));
    }

    // Run preprocessing
    if let Err(e) = run() {
        eprintln!("Error: {e:?}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let (ctx, book) = parse_input(io::stdin())?;
    let preprocessor = TermlinkPreprocessor::new(&ctx)?;
    let processed = preprocessor.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed)?;
    Ok(())
}
