mod stackcollapse;
mod flamegraph;
mod flgutils;
mod perfutils;

use std::{env, fs};

fn gen_html(cli_args: &[String]) {
    let in_filename = flgutils::get_floating_arg(cli_args)
        .unwrap_or("perf.data");
    let out_filename = flgutils::get_arg(cli_args, "-o")
        .unwrap_or("flamegraph.html");

    let raw_text = perfutils::from_file(in_filename);
    let stacks = stackcollapse::collapse_perf(
        &raw_text, 
        &stackcollapse::Options::default()
    );

    let html = flamegraph::generate_flamegraph(&stacks, "title", None);
    fs::write(out_filename, html).unwrap();
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
        _ => panic!("Invalid Arguments"),
    }

}
