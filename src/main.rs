mod stackcollapse;
mod flamegraph;
mod flgutils;
mod perfutils;

use std::{env, fs, path::Path};
use std::collections::HashMap;

fn gen_html(cli_args: &[String]) {
    let in_filename = flgutils::get_floating_arg(cli_args)
        .unwrap_or("perf.data");
    let out_filename = flgutils::get_arg(cli_args, "-o")
        .unwrap_or("flamegraph.html");

    // Use the input filename as the default title
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
}

fn gen_batch_html(cli_args: &[String]) {
    let in_filenames = flgutils::get_all_floating_args(cli_args);
    let out_filename = flgutils::get_arg(cli_args, "-o")
        .unwrap_or("flamegraphs.html");

    if in_filenames.is_empty() {
        eprintln!("Error: genbatch requires at least one input file");
        std::process::exit(1);
    }

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

    if entries.len() > 1 {
        entries.push(flamegraph::FlameGraphEntry { 
            stacks: combined_stacks, 
            title: "Combined".to_string() 
        });
    }

    let html = flamegraph::generate_batch_flamegraph(&entries);
    if let Err(e) = fs::write(out_filename, html) {
        eprintln!("Failed to write output to {}: {}", out_filename, e);
        std::process::exit(1);
    }
    
    eprintln!("Generated {} flamegraphs in {}", entries.len(), out_filename);
}

fn print_help() {
    println!("flg 0.1.0");
    println!("Usage:");
    println!("  flg gen <perf.data> [-o <output.html>]      Generate a single flamegraph");
    println!("  flg genbatch <perf.data>... [-o <output.html>] Generate multiple stacked flamegraphs");
    println!("  flg --help, -h                               Print this help message");
    println!("  flg --version, -v                            Print version information");
}

fn print_version() {
    println!("flg 0.1.0");
}

fn main() {
    let cli_args = env::args().collect::<Vec<String>>();
    
    if cli_args.len() <= 1 {
        print_help();
        return;
    }

    match cli_args[1].as_str() {
        "gen" => {
            gen_html(&cli_args[2..]);
        }
        "genbatch" => {
            gen_batch_html(&cli_args[2..]);
        }
        "--help" | "-h" => {
            print_help();
        }
        "--version" | "-v" => {
            print_version();
        }
        _ => {
            eprintln!("Invalid command: {}", cli_args[1]);
            print_help();
            std::process::exit(1);
        }
    }

}
