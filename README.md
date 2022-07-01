# Projclean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Find and clean dependencies & builds from software projects to saving space or making backup easier.

![screenshot](https://user-images.githubusercontent.com/4012553/176894927-1c3562b9-f8c3-4e85-9800-600abd746125.gif)

## Why

- **Save space**: Cleans unnecessary directories and files.
- **Very fast**: Written in rust, optimized for concurrency.
- **Easy to use**: A tui listing all found targets and pressing `<space>` to get rid of them.
- **Minified**: single executable file, no dependencies, no installation.

## Install

### With cargo

```
cargo install projclean
```

### Binaries on macOS, Linux, Windows

Download from [Github Releases](https://github.com/sigoden/projclean/releases), unzip and add projclean to your $PATH.

## CLI

```
USAGE:
    projclean [OPTIONS] [RULES]...

ARGS:
    <RULES>...    Search rules

OPTIONS:
    -C, --directory <DIR>    Start searching from DIR [default: .]
    -h, --help               Print help information
    -t, --targets            Print found targets, do not enter tui
    -V, --version            Print version information
```

Find and clearn node_modules folders.

```
projclean node_modules
```

Common search rules for common projects:

| name    | search rule                                    |
| :------ | :--------------------------------------------- |
| js      | `node_modules`                                 |
| rs      | `target@Cargo.toml`                            |
| vs      | `'^(Debug\|Release)$@\.sln$'`                  |
| ios     | `'^(build\|xcuserdata\|DerivedData)$@Podfile'` |
| android | `build@build.gradle`                           |
| java    | `target@pom.xml`                               |
| php     | `vendor@composer.json`                         |

Find and clean dependencies & builds from kinds of projects.

```
projclean node_modules target@Cargo.toml target@pom.xml
```

Start searching from specific directory, other than currenct work directory.

```
projclean -C $HOME node_modules target@pom.xml
```

## Search Rule

Projclean find targets according search rule.

Rule consist of two parts:

```
<target-folder>[@flag-file]
```

> Both target folder and flag file can be plain text or regex.

Flag file is used to filter out folders that match only names but not projects.
 
E.g. The directory has the following contents:

```
.
├── misc-proj
│   └── target
└── rust-proj
    ├── Cargo.toml
    └── target
```

Rule `target` found all `target` folders 

```
$ projclean -t target
/tmp/demo/rust-proj/target
/tmp/demo/misc-proj/target
```

Rule `target@Cargo.toml` found `target` folders belongs the rust project.

```
$ projclean -t target@Cargo.toml
/tmp/demo/rust-proj/target
```

## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.