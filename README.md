# ProjClean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Find and clean heavy build or cache directories. 

ProjClean finds directories such as node_modules(node), target(rust), build(java) and their storage space for you, so you can easily inspect or clean.

![screenshot](https://user-images.githubusercontent.com/4012553/157594166-74ea021b-2661-4799-993e-b3d80f369f4d.gif)

## Install

### With cargo

```
cargo install projclean
```

### Binaries on macOS, Linux, Windows

Download from [Github Releases](https://github.com/sigoden/projclean/releases), unzip and add projclean to your $PATH.

## Rule

ProjClean finds target folder according to project rule.

Each project rule consist of:

```
<target>[;flag][;name]
```
- target: folders to be searched, e.g. `node_modules`, `^(build|dist)$`
- flag: specific file to a specific project, e.g. `Cargo.toml` to rust, `build.gradle` to java or `\.sln$` to vs.
- name: rule name.

The flag is used to filter out folders that are not in the project.

## Usage

- Starting search from current directory

```
projclean
```

- Starting search from $HOME directory

```
projclean $HOME
```

- Print default rules

```
projclean -L
```
```
node_modules;;node
target;Cargo.toml;rust
build;build.gradle;java
^(Debug|Release)$;\.sln$;vs
```

- Use custom rules

Search tmp folder

```sh
projclean -r tmp
```

Search build or dist folder belongs to js project

```sh
projclean -r '^(build|dist)$;package.json;js'
# or
projclean -r 'build;package.json;js' -r 'dist;packge.json;js'
```

- Load custom rules from file

You can write the rules to a file for reuse.

```sh
projclean -L > rules
echo 'build;pom.xml;java' >> rules
projclean -f rules
```

- List found targets only, do not enter tui

```sh
projclean -t
projclean -t | xargs rm -rf
```

## License

Copyright (c) 2022 projclean-developers.

argc is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.