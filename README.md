# Projclean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Project dependecies & build artifacts cleanup tool.

![screenshot](https://github.com/sigoden/projclean/assets/4012553/a875c5f2-e87e-4b74-a48e-3636024d31eb)

## Why

- **Save space**: Cleans unnecessary directories and files.
- **Very fast**: Written in rust, optimized for concurrency.
- **Easy to use**: A tui listing all found targets and pressing `SPACE` to get rid of them.
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
Usage: projclean [OPTIONS] [RULES]...

Arguments:
  [RULES]...  Search rules, e.g. node_modules target@Cargo.toml

Options:
  -C, --cwd <DIR>         Start searching from <DIR> [default: .]
  -x, --exclude <DIR>     Exclude directories from search, e.g. ignore1,ignore2
  -t, --time <[+|-]DAY>   Path was last modified less than, more than or exactly <DAY> days
  -s, --size <[+|-]SIZE>  Path uses less than, more than or exactly <SIZE> units (K|M|G|T) of space
  -D, --delete-all        Automatically delete all found targets
  -P, --print             Print the found targets
  -h, --help              Print help
  -V, --version           Print version
```

Clean up node_modules.

```sh
projclean node_modules
```

Clean up various types of projects.

```sh
projclean node_modules target@Cargo.toml
```

Start searching from a specific directory with `-C` or `--cwd`

```sh
projclean -C $HOME node_modules # equal to `cd $HOME && projclean node_modules`
```

Find node_modules with the latest updates over 30 days and occupy more than 1G disk space.
```sh
projclean node_modules --time +30 --size +1G
```

## Search Rule

Projclean find targets according search rule.

Rule consist of two parts:

```
<target[,target...]>[@detect[,detect...]]
```

| project  | rule                                          |
| :------- | :-------------------------------------------- |
| nodejs   | `node_modules`                                |
| cargo    | `target@Cargo.toml`                           |
| maven    | `target@pom.xml`                              |
| gradle   | `.gradle,build@build.gradle,build.gradle.kts` |
| cmake    | `build@CMakeLists.txt`                        |
| composer | `vendor@composer.json`                        |
| dotnet   | `bin,obj@*.csproj,*.fsproj`                   |
| vs       | `.vs,Debug,Release@*.sln`                     |
| vc++     | `Debug,Release@*.vcxproj`                     |
| swift    | `.build,.swiftpm@Package.swift`               |
| pod      | `Pods@Podfile`                                |
| pub      | `.dart_tool,build@pubspec.yaml`               |
| sbt      | `target,project/target@build.sbt`             |
| jupyter  | `.ipynb_checkpoints@*.ipynb`                  |
| zig      | `zig-cache,zig-out@build.zig`                 |
| rebar    | `_build@rebar.config`                         |
| dune     | `_build@dune-project`                         |
| mix      | `_build@mix.exs`                              |
| stack    | `.stack-work@stack.yaml`                      |
| godot    | `.godot@project.godot`                        |

## License

Copyright (c) 2022-2024 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.