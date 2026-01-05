use std::process;
use std::fs;
use std::io::Read;

fn from_datafile(filepath: &str) -> String {
    let cmd = process::Command::new("perf")
        .args(["script", "-i", filepath])
        .output();

    let cmd = match cmd {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to spawn perf: {}", e);
            process::exit(1);
        },
    };

    match String::from_utf8(cmd.stdout) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to parse perf script output as UTF-8: {}", e);
            process::exit(1);
        }
    }
}

pub fn from_file(filepath: &str) -> String {
    let header = "PERFILE2";
    let mut buf = [0u8; 8];
    
    let mut fp = match fs::File::open(filepath) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to open {} => {}", filepath, e);
            process::exit(1);
        },
    };

    let bytes_read = match fp.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read header from {}: {}", filepath, e);
            process::exit(1);
        }
    };
    drop(fp);

    if bytes_read == 8 && &buf == header.as_bytes() {
        from_datafile(filepath)
    } else {
        match fs::read_to_string(filepath) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", filepath, e);
                process::exit(1);
            }
        }
    }
}