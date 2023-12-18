# Projclean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Project dependecies & build artifacts cleaner.

![screenshot](https://user-images.githubusercontent.com/4012553/192139216-6d76ea7b-6163-471a-b5bb-07ef465aa5b5.gif)

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
  -C, --cwd <DIR>      Start searching from <DIR> [default: .]
  -x, --exclude <DIR>  Exclude directories from search, e.g. ignore1,ignore2
  -D, --delete-all     Automatically delete all found targets
  -P, --print          Print the found targets
  -h, --help           Print help
  -V, --version        Print version
```

Clean up node_modules.

```
projclean node_modules
```

Clean up various types of projects.

```
projclean node_modules target@Cargo.toml
```

Start searching from a specific directory with `-C` or `--cwd`

```sh
projclean -C $HOME node_modules # equal to `cd $HOME && projclean node_modules`
```

## Search Rule

Projclean find targets according search rule.

Rule consist of two parts:

```
<target[,target...]>[@flag]
```

| project  | rule                                                                              |
| :------- | :-------------------------------------------------------------------------------- |
| node     | `node_modules`                                                                    |
| cargo    | `target@Cargo.toml`                                                               |
| maven    | `target@pom.xml`                                                                  |
| gradle   | `.gradle,build@build.gradle`                                                      |
| python   | `__pycache__,.mypy_cache,.pytest_cache,.ruff_cache,.tox@*.py`                     |
| composer | `vendor@composer.json`                                                            |
| swift    | `.build,.swiftpm@Package.swift`                                                   |
| dart     | `.dart_tool,build,linux/flutter/ephemeral,windows/flutter/ephemeral@pubspec.yaml` |
| sbt      | `target,project/target@build.sbt`                                                 |
| zig      | `zig-cache,zig-out@build.zig`                                                     |
| stack    | `.stack-work@stack.yaml`                                                          |
| jupyter  | `.ipynb_checkpoints@*.ipynb`                                                      |
| ocaml    | `_build@dune-project`                                                             |
| elixir   | `_build@mix.exs`                                                                  |
| erlang   | `_build@rebar.config`                                                             |
| vc       | `Debug,Release@*.vcxproj`                                                         |
| c#       | `bin,obj@*.csproj`                                                                |
| f#       | `bin,obj@*.fsproj`                                                                |
| unity    | `Library,Temp,Obj,Logs,MemoryCaptures,Build,Builds@Assembly-CSharp.csproj`        |
| unreal   | `Binaries,Build,Saved,DerivedDataCache,Intermediate@*.uproject`                   |
| godot    | `.godot@project.godot`                                                            |

## License

Copyright (c) 2022-2023 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.