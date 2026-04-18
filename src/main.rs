use crate::duckman_app::build_duckman_app;
use std::env;
use std::ffi::OsString;

mod duckman_app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut raw_args: Vec<OsString> = env::args_os().collect();
    // get sub command name
    let mut sub_command_name = "".to_owned();
    if (raw_args.len() > 1) {
        let arg_1 = raw_args[1].clone().to_str().unwrap().to_owned();
        if arg_1.starts_with('-') {
            if raw_args.len() > 3 {
                sub_command_name = raw_args[3].clone().to_str().unwrap().to_owned();
            }
        } else {
            sub_command_name = arg_1;
        }
        print!("sub command: {}", sub_command_name);
    }
    let app = build_duckman_app();
    let matches = app.get_matches_from(raw_args);
    Ok(())
}
