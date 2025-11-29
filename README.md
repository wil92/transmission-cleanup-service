# Transmission Cleanup Service

This service is designed to automatically clean up completed downloads in Transmission, a popular BitTorrent client. It
removes torrents and their associated files after a specified period of time, helping to free up disk space and keep
your Transmission client organized.

## Features

- Automatically removes completed torrents after a configurable delay.
- Supports configuration via environment variables.
- Logs actions for easy monitoring.
- Runs as a background service.
- Compatible with Docker for easy deployment.

## Configuration

### Environment Variables

The service can be configured using the following environment variables:
- `FP_MONITORING_URL`: URL for monitoring service (mandatory).
- `FP_DATABASE_PATH`: Path to the sqlite database file (if not set the application save the data in the memory).
- `FP_SCAN_INTERVAL`: Interval (in seconds) between scans for the list of downloads in the transmission client (default: 1m).
- `FP_FILE_LIFETIME`: Time (in seconds) after which downloads will be removed. Used to clean endless downloads (default: 7d).
- `FP_FILE_LIFETIME_AFTER_COPIED`: Time (in seconds) after which completed downloads will be removed (default: 5h).
- `FP_USERNAME`: Transmission username.
- `FP_PASSWORD`: Transmission password.

### Arguments

```shell
Usage: program [options]
Options:
  -h, --help                          Show this help message and exit
  -d, --monitoring-url                Specify the monitoring url
                                      [env: FP_MONITORING_URL]
  -p, --database-path PATH            Specify the database path
                                      [env: FP_DATABASE_PATH]
  -s, --scan-interval                 Specify the scan interval in seconds
                                      [env: FP_SCAN_INTERVAL]
  -l, --file-lifetime                 Specify the files lifetime in seconds
                                      [env: FP_FILE_LIFETIME]
  -a, --file-lifetime-after-copied    Specify the files lifetime after copied in seconds
                                      [env: FP_FILE_LIFETIME_AFTER_COPIED]
  -u, --username                      Specify the username for authetication
                                      [env: FP_USERNAME]
  -p, --password                      Specify the password for authetication
                                      [env: FP_PASSWORD]
```

## Deployment with Docker

```shell
docker build -t transmission-cleanup-service .
```

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests on the GitHub repository.
