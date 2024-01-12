#![allow(dead_code)]

use anyhow::Result;
use assert_cmd::prelude::*;
use assert_fs::fixture::TempDir;
use assert_fs::prelude::*;
use std::process::Command;

pub const PATHS: [&str; 20] = [
    "nodejs/node_modules/",
    "cargo/target/",
    "cargo/src/",
    "cargo/Cargo.toml",
    "cargo-not/target/",
    "gradle/.gradle/",
    "gradle/build/",
    "gradle/build.gradle",
    "gradle-kts/.gradle/",
    "gradle-kts/build/",
    "gradle-kts/build.gradle.kts",
    "dotnet-cs/bin",
    "dotnet-cs/obj",
    "dotnet-cs/App.csproj",
    "dotnet-fs/bin",
    "dotnet-fs/obj",
    "dotnet-fs/App.fsproj",
    "mixed/_build",
    "mixed/rebar.config",
    "mixed/dune-project",
];

pub fn search(tmpdir: TempDir, rules: &[&str]) -> Result<Vec<String>> {
    let name = tmpdir.file_name().unwrap().to_string_lossy().to_string();
    let output = Command::cargo_bin("projclean")
        .expect("Couldn't find test binary")
        .current_dir(tmpdir.path())
        .arg("-P")
        .args(rules)
        .output()?;
    let output = std::str::from_utf8(&output.stdout)?.trim().to_string();
    let mut paths: Vec<String> = output
        .split('\n')
        .map(|path| {
            let path = if let Some(idx) = path.find(&name) {
                &path[(idx + name.len() + 1)..]
            } else {
                path
            };
            path.replace('\\', "/").to_string()
        })
        .collect();
    paths.sort_unstable();
    Ok(paths)
}

pub fn tmpdir() -> TempDir {
    let tmpdir = assert_fs::TempDir::new().expect("Couldn't create a temp dir for tests");
    for path in PATHS {
        if path.ends_with('/') {
            tmpdir.child(path).create_dir_all().unwrap();
        } else {
            tmpdir.child(path).write_str("").unwrap();
        }
    }
    tmpdir
}
