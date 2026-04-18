use std::env;
use std::ffi::OsString;

fn main() {
    let mut raw_args: Vec<OsString> = env::args_os().collect();
    // get shim command name
    let shim_command = raw_args[0].clone().to_str().unwrap().to_owned();
    println!("shim command: {}", shim_command);
}
