# Projclean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Clean project-based cache dependencies and build outputs.

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
    <RULES>...    Search rules, like node_modules or target@Cargo.toml

OPTIONS:
    -C, --cwd <DIR>    Start searching from DIR [default: .]
    -f, --force        Delete found targets without entering tui
    -p, --print        Print found targets only
    -h, --help         Print help information
    -V, --version      Print version information
```

Clean up node_modules.

```
projclean node_modules
```

Clean up multiple kinds of projects.

```
projclean node_modules target@Cargo.toml
```

Start searching from a specific directory with `-C` or `--cwd`

```
projclean -C $HOME node_modules
```

Enter interactive mode to select rules when invoking `projclean` without any rule.

```
projclean
? Select search rules:  
> [ ] js              node_modules
  [ ] rust            target@Cargo.toml
  [ ] vs              ^(Debug|Release)$@\.sln$
  [ ] ios             ^(build|xcuserdata|DerivedData)$@Podfile
  [ ] android         build@build.gradle
  [ ] java            target@pom.xml
  [ ] php             vendor@composer.json
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
$ projclean target -p
/tmp/demo/rust-proj/target
/tmp/demo/misc-proj/target
```

Rule `target@Cargo.toml` found `target` folders belongs the rust project.

```
$ projclean target@Cargo.toml -p
/tmp/demo/rust-proj/target
```

## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.