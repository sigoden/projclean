# ProjClean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Projclean: find and clean project build/cache for saving disk or making backups/rsync easier.

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
SAGE:
    projclean [OPTIONS] [--] [PATH]

ARGS:
    <PATH>    Start searching from

OPTIONS:
    -h, --help              Print help information
    -r, --rule <RULE>...    Add a search rule
    -t, --list-targets      List found targets and exit
    -V, --version           Print version information
```

Find node_modules folders.

```
projclean -r node_modules
```

Find node_modules folders then print.

```
projclean -r node_modules -t
```

Find node_modules folders starting from $HOME.

```
projclean $HOME -r node_modules
```

Find node_modules folders and rust target folders.

```
projclean $HOME -r node_modules -r target@Cargo.toml
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

| project | command                                                     |
| ------- | ----------------------------------------------------------- |
| js      | `projclean -r node_modules`                                 |
| rs      | `projclean -r target@Cargo.toml`                            |
| vs      | `projclean -r '^(Debug\|Release)$@\.sln$'`                  |
| ios     | `projclean -r '^(build\|xcuserdata\|DerivedData)$@Podfile'` |
| android | `projclean -r build@build.gradle`                           |


## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.