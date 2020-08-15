# Lares: Minimal Fever API Implementation

[![Build Status](https://github.com/fanzeyi/lares/workflows/.github/workflows/buildtest.yml/badge.svg)](https://github.com/fanzeyi/lares/actions?query=workflow%3A%22Build+%26+Test%22) [![Crates.io](https://img.shields.io/crates/v/lares)](https://crates.io/crates/lares)

**Lares** is a minimal [Fever API](https://feedafever.com/api) implementation
written in Rust. It aims to provide a RSS reader backend with zero setup. It
uses SQLite 3 as storage engine. **It does not provide an user interface.**

It is recommended to use Reeder as client to lares.

## Install

```
cargo install lares
```

_Binary package will be provided in the future._

## Usage

Lares consists of two parts, CLI and server. Feeds and groups are only
manageable via the command line interface.

```
$ lares --help
lares 0.1.0
Minimal RSS service

USAGE:
    lares <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    feed      Manages feeds
    group     Manages group
    help      Prints this message or the help of the given subcommand(s)
    server    Starts web server
```

Or, to start a server:

```
$ lares server -H 127.0.0.1 -p 4000
```

## License

MIT