pub struct Args {
    pub monitoring_url: Option<String>,
    pub database_path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub scan_interval: Option<u32>,
    pub file_lifetime: Option<u32>,
    pub file_lifetime_after_copied: Option<u32>,
}

impl Args {
    pub fn new(args: Vec<String>) -> Self {
        let mut args_ins = Args {
            monitoring_url: None,
            database_path: None,
            scan_interval: None,
            file_lifetime: None,
            file_lifetime_after_copied: None,
            username: None,
            password: None,
        };

        // parse command line arguments
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--help" | "-h" => {
                    println!("Usage: program [options]");
                    println!("Options:");
                    println!(
                        "  -h, --help                          Show this help message and exit"
                    );
                    println!("  -d, --monitoring-url                Specify the monitoring url");
                    println!("                                      [env: FP_MONITORING_URL]");
                    println!("  -p, --database-path PATH            Specify the database path");
                    println!("                                      [env: FP_DATABASE_PATH]");
                    println!(
                        "  -s, --scan-interval                 Specify the scan interval in seconds"
                    );
                    println!("                                      [env: FP_SCAN_INTERVAL]");
                    println!(
                        "  -l, --file-lifetime                 Specify the files lifetime in sec"
                    );
                    println!("                                      [env: FP_FILE_LIFETIME]");
                    println!(
                        "  -a, --file-lifetime-after-copied    Specify the files lifetime after copied"
                    );
                    println!(
                        "                                      [env: FP_FILE_LIFETIME_AFTER_COPIED]"
                    );
                    println!(
                        "  -u, --username                      Specify the username for authe"
                    );
                    println!("                                      [env: FP_USERNAME]");
                    println!(
                        "  -p, --password                      Specify the password for authe"
                    );
                    println!("                                      [env: FP_PASSWORD]");
                    std::process::exit(0);
                }
                "-m" | "--monitoring-directory" => {
                    args_ins.monitoring_url = Self::next_value(&args, &mut i);
                }
                "-d" | "--database-path" => {
                    args_ins.database_path = Self::next_value(&args, &mut i);
                }
                "-s" | "--scan-interval" => {
                    args_ins.scan_interval =
                        Self::next_value(&args, &mut i).and_then(|v| v.parse::<u32>().ok());
                }
                "-l" | "--file-lifetime" => {
                    args_ins.file_lifetime =
                        Self::next_value(&args, &mut i).and_then(|v| v.parse::<u32>().ok());
                }
                "-a" | "--file-lifetime-after-copied" => {
                    args_ins.file_lifetime_after_copied =
                        Self::next_value(&args, &mut i).and_then(|v| v.parse::<u32>().ok());
                }
                "-u" | "--username" => {
                    args_ins.username = Self::next_value(&args, &mut i);
                }
                "-p" | "--password" => {
                    args_ins.password = Self::next_value(&args, &mut i);
                }
                _ => {}
            }
            i += 1;
        }

        // parse environment variables if needed (not implemented here)
        if args_ins.monitoring_url.is_none() {
            if let Ok(dir) = std::env::var("FP_MONITORING_URL") {
                args_ins.monitoring_url = Some(dir);
            }
        }
        if args_ins.database_path.is_none() {
            if let Ok(db_path) = std::env::var("FP_DATABASE_PATH") {
                args_ins.database_path = Some(db_path);
            }
        }
        if args_ins.scan_interval.is_none() {
            if let Ok(interval) = std::env::var("FP_SCAN_INTERVAL") {
                if let Ok(parsed) = interval.parse::<u32>() {
                    args_ins.scan_interval = Some(parsed);
                }
            }
        }
        if args_ins.file_lifetime.is_none() {
            if let Ok(lifetime) = std::env::var("FP_FILE_LIFETIME") {
                if let Ok(parsed) = lifetime.parse::<u32>() {
                    args_ins.file_lifetime = Some(parsed);
                }
            }
        }
        if args_ins.file_lifetime_after_copied.is_none() {
            if let Ok(lifetime) = std::env::var("FP_FILE_LIFETIME_AFTER_COPIED") {
                if let Ok(parsed) = lifetime.parse::<u32>() {
                    args_ins.file_lifetime_after_copied = Some(parsed);
                }
            }
        }
        if args_ins.username.is_none() {
            if let Ok(user) = std::env::var("FP_USERNAME") {
                args_ins.username = Some(user);
            }
        }
        if args_ins.password.is_none() {
            if let Ok(pass) = std::env::var("FP_PASSWORD") {
                args_ins.password = Some(pass);
            }
        }

        args_ins
    }

    fn next_value(args: &Vec<String>, index: &mut usize) -> Option<String> {
        *index += 1;
        if *index < args.len() {
            Some(args[*index].clone())
        } else {
            None
        }
    }

    pub fn validate(&self) -> bool {
        self.monitoring_url.is_some() && self.username.is_some() && self.password.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = vec![
            "program".to_string(),
            "-m".to_string(),
            "/path/to/dir".to_string(),
            "-d".to_string(),
            "/path/to/db".to_string(),
            "-s".to_string(),
            "120".to_string(),
            "-l".to_string(),
            "7200".to_string(),
            "-a".to_string(),
            "3600".to_string(),
            "-u".to_string(),
            "user".to_string(),
            "-p".to_string(),
            "pass".to_string(),
        ];
        let parsed_args = Args::new(args);
        assert_eq!(parsed_args.monitoring_url, Some("/path/to/dir".to_string()));
        assert_eq!(parsed_args.database_path, Some("/path/to/db".to_string()));
        assert_eq!(parsed_args.scan_interval, Some(120));
        assert_eq!(parsed_args.file_lifetime, Some(7200));
        assert_eq!(parsed_args.file_lifetime_after_copied, Some(3600)); // new assertion
        assert_eq!(parsed_args.username, Some("user".to_string()));
        assert_eq!(parsed_args.password, Some("pass".to_string()));
    }

    #[test]
    fn test_args_parsing_with_wrong_number() {
        let args = vec![
            "program".to_string(),
            "-m".to_string(),
            "/path/to/dir".to_string(),
            "-a".to_string(),
            "3600".to_string(),
            "-s".to_string(),
            "not_a_number".to_string(),
            "-l".to_string(),
            "7200".to_string(),
        ];
        let parsed_args = Args::new(args);
        assert_eq!(parsed_args.monitoring_url, Some("/path/to/dir".to_string()));
        assert_eq!(parsed_args.scan_interval, None);
        assert_eq!(parsed_args.file_lifetime, Some(7200));
        assert_eq!(parsed_args.file_lifetime_after_copied, Some(3600)); // ensure default is None
    }

    #[test]
    fn test_args_parsing_with_env_vars() {
        unsafe {
            std::env::set_var("FP_MONITORING_URL", "http://some.com");
            std::env::set_var("FP_FILE_LIFETIME", "7500");
            std::env::set_var("FP_FILE_LIFETIME_AFTER_COPIED", "3600");
        }

        let args = vec!["program".to_string(), "-s".to_string(), "150".to_string()];
        let parsed_args = Args::new(args);
        assert_eq!(
            parsed_args.monitoring_url,
            Some("http://some.com".to_string())
        );
        assert_eq!(parsed_args.scan_interval, Some(150));
        assert_eq!(parsed_args.file_lifetime, Some(7500));
        assert_eq!(parsed_args.file_lifetime_after_copied, Some(3600));
    }
}
