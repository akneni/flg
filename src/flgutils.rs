use std::process;

/// Gets the value of an command line argument that's "floating": not a flag itself and
/// not the argument for a flag. 
/// This enables a gcc like interface (`gcc main.c -o main` or `gcc -o main main.c` both work)
pub fn get_floating_arg<'a>(cli_args: &'a[String]) -> Option<&'a str> {
    for i in 0..cli_args.len() {
        if !cli_args[i].starts_with("-") {
            if i == 0 || !cli_args[i-1].starts_with("-") {
                return Some(cli_args[i].as_str());
            }
        }
    }
    None
}

pub fn get_arg<'a>(cli_args: &'a[String], flag: &str) -> Option<&'a str> {
    for i in 0..cli_args.len() {
        if cli_args[i] == flag {
            if i + 1 < cli_args.len() {
                return Some(cli_args[i+1].as_str());
            } else {
                eprintln!("flag {} passed without any arguments", flag);
                process::exit(1);
            }
        }
    }
    None
}