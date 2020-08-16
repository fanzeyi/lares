# Lares: Minimal Fever API Implementation

[![Build Status](https://github.com/fanzeyi/lares/workflows/Build%20&%20Test/badge.svg)](https://github.com/fanzeyi/lares/actions?query=workflow%3A%22Build+%26+Test%22)
[![Crates.io](https://img.shields.io/crates/v/lares)](https://crates.io/crates/lares)
[![Docker Status](https://img.shields.io/docker/cloud/build/fanzeyi/lares.svg)](https://hub.docker.com/r/fanzeyi/lares)

**Lares** is a minimal [Fever API](https://feedafever.com/api) implementation
written in Rust. It aims to provide a RSS reader backend with zero setup. It
uses SQLite 3 as storage engine. **It does not provide an user interface.**

It is recommended to use Reeder as client to lares.

## Install

**Cargo**

```
cargo install lares
```

**Docker**

```
docker run -it fanzeyi/lares
```

**Pre-built Binary**

For each release, GitHub Action will attach pre-built binaries for Ubuntu,
macOS and Windows. You can find these binaries in the [release
page](https://github.com/fanzeyi/lares/releases).

## Usage

Lares consists of two parts, CLI and server. Feeds and groups are only
manageable via the command line interface.

```
$ lares --help
lares 0.1.1
Minimal RSS service

USAGE:
    lares [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --database <database>     [default: lares.db]

SUBCOMMANDS:
    feed      Manages feeds
    group     Manages group
    help      Prints this message or the help of the given subcommand(s)
    server    Starts web server
```

Or, to start a server:

```
$ lares server --help
lares-server 0.1.1
Starts web server

USAGE:
    lares server [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -H, --host <host>            Specifies host of server [default: 127.0.0.1]
    -P, --password <password>    Specifies password used in authentication
    -p, --port <port>            Specifies port of server [default: 4000]
    -u, --username <username>    Specifies username used in authentication
```

To start a lares server listens to `127.0.0.1:4000` that only accepts
authentication with `lares` and `apassword` and placing the SQLite database
at `/var/lares.db`, run:

```
$ lares --database /var/lares.db server --host 127.0.0.1 --port 4000 \
  --username lares --password apassword
```

## License

MIT