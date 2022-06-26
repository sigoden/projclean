# ProjClean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Project non-essential files cleaner. 

![screenshot](https://user-images.githubusercontent.com/4012553/172361654-5fa36424-10da-4c52-b84a-f44c27cb1a17.gif)

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
SAGE:
    projclean [OPTIONS] [--] [PATH]

ARGS:
    <PATH>    Start searching from

OPTIONS:
    -h, --help              Print help information
    -r, --rule <RULE>...    Add a search rule
    -t, --targets           Print found target
    -V, --version           Print version information
```

Find node_modules folders.

```
projclean -r node_modules
```

Find node_modules folders starting from $HOME.

```
projclean $HOME -r node_modules
```

Find node_modules folders and rust target folders.

```
projclean -r node_modules -r target@Cargo.toml
```

## Rule

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
$ projclean -t -r target
/tmp/demo/rust-proj/target
/tmp/demo/misc-proj/target
```

Rule `target@Cargo.toml` found `target` folders belongs the rust project.

```
$ projclean -t -r target@Cargo.toml
/tmp/demo/rust-proj/target
```

## Projects

Common search rules for common projects:

| name    | command                                           |
| :------ | :------------------------------------------------ |
| js      | `-r node_modules`                                 |
| rs      | `-r target@Cargo.toml`                            |
| vs      | `-r '^(Debug\|Release)$@\.sln$'`                  |
| ios     | `-r '^(build\|xcuserdata\|DerivedData)$@Podfile'` |
| android | `-r build@build.gradle`                           |
| java    | `-r target@pom.xml`                               |
| php     | `-r vendor@composer.json`                         |

## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.