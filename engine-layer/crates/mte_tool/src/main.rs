use std::env;

mod process_assets;
mod rs_parser;
mod types;
mod utils;
mod workmods;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    let mut release = false;
    let workmod;

    match args[1].as_str() {
        "prepare" => workmod = WorkMod::Prepare,
        "process" => workmod = WorkMod::Process,
        _ => {
            return Err(
                "Выберите один из трёх режимов работы:\nprepare\nprocess\nclean".to_string(),
            );
        }
    }

    if let Some(s) = args.get(2) {
        if s.as_str() == "--release" {
            release = true;
        }
    }

    match workmod {
        WorkMod::Prepare => workmods::prepare::prepare(&args[2..], release)?,
        WorkMod::Process => workmods::process::process(&args[2..], release)?,
    };

    Ok(())
}

enum WorkMod {
    Prepare,
    Process,
}
