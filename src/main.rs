use std::env;

use fp::Monitor;

mod args;

#[tokio::main]
async fn main() {
    let args_values = args::Args::new(env::args().collect());

    if args_values.validate() {
        let mut monitor = Monitor::new(
            args_values.monitoring_url.unwrap().as_str(),
            args_values.database_path,
            args_values.scan_interval,
            args_values.file_lifetime,
            args_values.file_lifetime_after_copied,
            args_values.username.unwrap().as_str(),
            args_values.password.unwrap().as_str(),
        );
        monitor.run(None).await;
    } else {
        eprintln!("Invalid arguments provided. Please check help with -h.");
    }
}
