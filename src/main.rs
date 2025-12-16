mod stackcollapse;
mod flamegraph;
mod flgutils;
mod perfutils;

use std::{env, fs, path::Path};

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
    fs::write(out_filename, html).unwrap();
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

        entries.push(flamegraph::FlameGraphEntry { stacks, title });
    }

    let html = flamegraph::generate_batch_flamegraph(&entries);
    fs::write(out_filename, html).unwrap();
    
    eprintln!("Generated {} flamegraphs in {}", entries.len(), out_filename);
}

fn main() {
    let cli_args = env::args().collect::<Vec<String>>();
    
    if cli_args.len() <= 1 {
        panic!("Invalid Arguments");
    }

    match cli_args[1].as_str() {
        "gen" => {
            gen_html(&cli_args[2..]);
        }
        "genbatch" => {
            gen_batch_html(&cli_args[2..]);
        }
        _ => panic!("Invalid Arguments"),
    }

}
