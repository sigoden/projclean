# ProjClean

[![CI](https://github.com/sigoden/projclean/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/projclean/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/projclean.svg)](https://crates.io/crates/projclean)

Find and clean heavy build or cache directories. 

ProjClean finds directories such as node_modules(node), target(rust), build(java) and their storage space for you, so you can easily clean them up.

![screenshot](https://user-images.githubusercontent.com/4012553/157594166-74ea021b-2661-4799-993e-b3d80f369f4d.gif)


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
<target name>[;feature file][;project name]
```
- target name: the name of the directory or file to be cleaned, a required field, and supports regular expressions.
- feature file: a file unique to the project, optional, supports regularization.
- project name: a remark, optional.

If you simply pass the directory name, you may find a lot of irrelevant directories.
It is more accurate to specify a project using a feature file, requiring that the directory must be in the project.

The default project rules are:
```
node_modules;;node
target;Cargo.toml;rust
build;build.gradle;java
^(Debug|Release)$;\.sln$;vs
```

You can set custom rule with `-p --project`.

```sh
projclean -p dist -p '.next;;nextjs' -p '^(build|dist)$;package.json;js'
```

You can also use a rules file

```sh
echo dist > rules
echo '.next;;nextjs' >> rules
echo '^(build|dist)$;package.json;js' >> rules
projclean -f rules
```

Other options are used as follows:

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