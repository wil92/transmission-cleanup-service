use std::env;

use fp::Monitor;

mod args;

fn main() {
    let args_values = args::Args::new(env::args().collect());

    if args_values.validate() {
        let mut monitor = Monitor::new(
            args_values.monitoring_url.expect("Error with monitoring_url param").as_str(),
            args_values.database_path,
            args_values.scan_interval,
            args_values.file_lifetime,
            args_values.file_lifetime_after_copied,
            args_values.username.expect("Error with username param").as_str(),
            args_values.password.expect("Error with password param").as_str(),
        );
        monitor.run(None);
    } else {
        eprintln!("Invalid arguments provided. Please check help with -h.");
    }
}
