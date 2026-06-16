![Greenbone Logo](https://www.greenbone.net/wp-content/uploads/gb_new-logo_horizontal_rgb_small.png)

# Greenbone Web Application Scanner (WAS) <!-- omit in toc -->

Greenbone WAS is a wrapper for the web application vulnerability scanner
*[Zed Attack Proxy (ZAP)][ZAP]* that offers an API
based on the [openvasd scanner API](https://greenbone.github.io/scanner-api/)
to run scans and retrieve results.

- [Installation](#installation)
- [Configuration](#configuration)
- [Maintainer](#maintainer)
- [Contributing](#contributing)
- [License](#license)

## Installation

The project contains the `greenbone-was` application which implements an
openvasd scanner interface for the [ZAP web application scanner][ZAP].
It is implemented in [Rust] and requires [cargo] for building and installing.

```sh
make DESTDIR=path/to/install install
```

The binary can be found at `path/to/install/usr/local/bin` afterwards.

## Configuration

WAS reads configuration from environment variables prefixed with
`GREENBONE_WAS_`. Values may also be provided through a `.env` file in the
working directory. Environment variables override the built-in defaults.

| Environment variable                             | Default                                  | Description                                                                                                                                                                                  |
| ------------------------------------------------ | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GREENBONE_WAS_LOG_FORMAT`                       | `fmt`                                    | Logging output format. Supported values are text-style `fmt` and `json`.                                                                                                                     |
| `GREENBONE_WAS_LOG_LEVEL`                        | `info`                                   | Logging filter level, for example `debug`, `info`, `warn`, or `error`.                                                                                                                       |
| `GREENBONE_WAS_PORT`                             | `8030`                                   | HTTP server listen port. The value must be a valid port number.                                                                                                                              |
| `GREENBONE_WAS_STORAGE_BACKEND`                  | `sqlite`                                 | Runtime storage backend. `sqlite` is currently the only valid value.                                                                                                                         |
| `GREENBONE_WAS_VAR_DATA_DIR`                     | `/var/lib/greenbone-was`                 | Directory for variable runtime data. When `GREENBONE_WAS_SQLITE_URL` is unset, the default database is created as `scans.db` in this directory.                                              |
| `GREENBONE_WAS_SQLITE_URL`                       | `sqlite:/var/lib/greenbone-was/scans.db` | Explicit SQLite connection URL. When set, this overrides `GREENBONE_WAS_VAR_DATA_DIR`. Runtime SQLite URLs must be file-backed; `sqlite::memory:` and URLs using `mode=memory` are rejected. |
| `GREENBONE_WAS_ZAP_BASE_URL`                     | `http://127.0.0.1:8547`                  | Base URL of the ZAP HTTP API.                                                                                                                                                                |
| `GREENBONE_WAS_ZAP_API_KEY`                      | `test-api-key`                           | API key used for authenticated ZAP API calls.                                                                                                                                                |
| `GREENBONE_WAS_SCAN_WORKER_COUNT`                | `1`                                      | Maximum number of concurrently running scan workers. The value must be greater than `0`.                                                                                                     |
| `GREENBONE_WAS_SCAN_ALERT_POLL_INTERVAL_SECONDS` | `10`                                     | Interval, in seconds, between ZAP alert polling attempts during active scans. The value must be greater than `0`.                                                                            |
| `GREENBONE_WAS_SCAN_STOP_GRACE_PERIOD_SECONDS`   | `300`                                    | Grace period, in seconds, to wait for a running scan to stop before forcing it to failed. The value must be greater than `0`.                                                                |
| `GREENBONE_WAS_SCAN_RETRY_MAX_RETRIES`           | `10`                                     | Maximum number of retry attempts for transient ZAP or storage failures.                                                                                                                      |
| `GREENBONE_WAS_SCAN_RETRY_MAX_DELAY_SECONDS`     | `60`                                     | Maximum backoff delay, in seconds, between retry attempts. The value must be greater than `0`.                                                                                               |

With the default storage configuration, WAS uses the persistent SQLite database
URL `sqlite:/var/lib/greenbone-was/scans.db`. Set
`GREENBONE_WAS_VAR_DATA_DIR` to move that default database location, or set
`GREENBONE_WAS_SQLITE_URL` when a fully explicit file-backed SQLite URL is
required.


## Maintainer

This project is maintained by [Greenbone AG][Greenbone].

## Contributing

Your contributions are highly appreciated. Please [create a pull
request](https://github.com/greenbone/web-appliaction-scanner/pulls) on GitHub.
Bigger changes need to be discussed with the development team via the
[issues section at github](https://github.com/greenbone/web-application-scanner/issues)
first.

## License

Copyright (C) 2026 [Greenbone AG][Greenbone]

Licensed under the [GNU Affero General Public License v3.0 or later](LICENSE).

[Greenbone]: https://www.greenbone.net/
[Rust]: https://rust-lang.org/
[cargo]: https://doc.rust-lang.org/stable/cargo/
[ZAP]: https://www.zaproxy.org/
