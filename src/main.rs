mod stackcollapse;
mod flamegraph;
mod perfutils;
mod cli;

use std::{fs, path::Path};
use std::collections::HashMap;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gen { output, files } => {
            gen_flamegraphs(&files, &output);
        }
    }
}

fn gen_flamegraphs(files: &[String], out_filename: &str) {
    let in_filenames: Vec<&str> = files.iter().map(|s| s.as_str()).collect();

    // Single file: generate simple flamegraph
    if in_filenames.len() == 1 {
        let in_filename = in_filenames[0];
        let default_title = Path::new(in_filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Flamegraph");

        let raw_text = perfutils::from_file(in_filename);
        let stacks = stackcollapse::collapse_perf(
            &raw_text, 
            &stackcollapse::Options::default()
        );

        let html = flamegraph::generate_flamegraph(&stacks, default_title, None);
        if let Err(e) = fs::write(out_filename, html) {
            eprintln!("Failed to write output to {}: {}", out_filename, e);
            std::process::exit(1);
        }
        return;
    }

    // Multiple files: generate batch flamegraph with Combined
    let mut entries = Vec::new();
    let mut combined_stacks = HashMap::new();

    for in_filename in &in_filenames {
        let title = Path::new(in_filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Flamegraph")
            .to_string();

        let raw_text = perfutils::from_file(in_filename);
        let stacks = stackcollapse::collapse_perf(
            &raw_text, 
            &stackcollapse::Options::default()
        );

        // Merge into combined stacks
        for (stack, count) in &stacks {
            *combined_stacks.entry(stack.clone()).or_insert(0) += count;
        }

        entries.push(flamegraph::FlameGraphEntry { stacks, title });
    }

    // Add combined flamegraph (always present when 2+ files)
    entries.push(flamegraph::FlameGraphEntry { 
        stacks: combined_stacks, 
        title: "Combined".to_string() 
    });

    let html = flamegraph::generate_batch_flamegraph(&entries);
    if let Err(e) = fs::write(out_filename, html) {
        eprintln!("Failed to write output to {}: {}", out_filename, e);
        std::process::exit(1);
    }
    
    eprintln!("Generated {} flamegraphs in {}", entries.len(), out_filename);
}
