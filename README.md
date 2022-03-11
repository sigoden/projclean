# ProjClean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Find and clean heavy build or cache directories.

![screenshot](https://user-images.githubusercontent.com/4012553/157594166-74ea021b-2661-4799-993e-b3d80f369f4d.gif)


ProjClean identifies projects based on the project feature file, and then decides based on that project whether or not the matching directory should be added to the cleanup list.

- Identify `Node` projects based on `package.json`, clean `node_modules` 
- Identify `Rust` projects based on `Cargo.toml`, clean `target`.
- Identify `Java/Android` projects based on `build.gradle`, clean `build`.
- Identify `Visutal Studio` projects based on `*.sln`, clean `Debug` `Release`.

## Install

### With cargo

```
cargo install projclean
```

### Binaries on macOS, Linux, Windows

Download from [Github Releases](https://github.com/sigoden/projclean/releases), unzip and add projclean to your $PATH.


## Project Rule

ProjClean finds target folders according to project rule.

Each project rule consist of three parts.
```
<to clean up directory>[;feature file][;project name]
```

The default project rules are:
```
node_modules;package.json;node
target;Cargo.toml;rust
build;build.gradle;java
^(Debug|Release)$;\.sln$;vs
```

You can append custom rules.

```sh
projclean -p dist -p '.next;;nextjs' -p '^(build|dist)$;package.json;js'
```

You can also write project rules to a file then load.

```sh
projclean -l > rules.csv
echo '.next;;nextjs' >> rules.csv
echo '^(build|dist)$;package.json;js' >> rules.csv

projclean -f rules.csv
```

More examples:

```sh
projclean                    # Find from current directory
projclean $HOME              # Find from $HOME directory
projclean -l                 # Print project rules
projclean -t                 # Print the matching directory directly (without entering tui)
```

## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.