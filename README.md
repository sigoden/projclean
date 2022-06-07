# ProjClean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Project cache finder and cleaner.

![screenshot](https://user-images.githubusercontent.com/4012553/172361654-5fa36424-10da-4c52-b84a-f44c27cb1a17.gif)

## Install

### With cargo

```
cargo install projclean
```

### Binaries on macOS, Linux, Windows

Download from [Github Releases](https://github.com/sigoden/projclean/releases), unzip and add projclean to your $PATH.

## Cli

```
USAGE:
    projclean [OPTIONS] [--] [PATH]

ARGS:
    <PATH>    Start searching from

OPTIONS:
    -h, --help              Print help information
    -r, --rule <RULE>...    Add a search rule
    -t, --list-targets      List found targets and exit
    -V, --version           Print version information
sigo ~/w/projclean$ 
```

Find node_modules folders

```
projclean -r node_modules
```

Find target folders for rust project

```
projclean -r target@Cargo.toml
```

Find node_modules folders then print

```
projclean -r node_modules -t
```

## Rule

Projclean detect project and target folder according to rule.

Rule consist of two parts:

```
<target folder>[@flag file]
```

> Both target folder and flag file can be regex.

If flag file is provided, Only folders with flag files will be selected.

For example. The current directory structure is as follows:

```
.
├── projA
│   ├── Cargo.toml
│   └── target
└── projB
    └── target
```

Run `projclean -r target` will find both `projA/target` and `projB/target`.

Run `projclean -r target@Cargo.toml` will find  `projA/target` only.

## Project

Below is a collection of rule of common porjects:

- js: `node_modules`
- rs: `target@Cargo.toml`
- vs: `^(Debug|Release)$@\.sln$`
- ios: `^(build|xcuserdata|DerivedData)$@Podfile`
- android: `build@build.gradle`

## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.