use fixtures::search;

use crate::fixtures::tmpdir;

mod fixtures;

#[test]
fn plain() {
    assert_eq!(
        search(tmpdir(), &["target@Cargo.toml"]).unwrap(),
        vec!["cargo/target"]
    );
}

#[test]
fn no_detect() {
    assert_eq!(
        search(tmpdir(), &["node_modules"]).unwrap(),
        vec!["nodejs/node_modules"]
    );
}

#[test]
fn multiple_detects() {
    assert_eq!(
        search(tmpdir(), &[".gradle,build@build.gradle,build.gradle.kts"]).unwrap(),
        vec![
            "gradle-kts/.gradle",
            "gradle-kts/build",
            "gradle/.gradle",
            "gradle/build"
        ]
    );
}

#[test]
fn detects_with_asterisk() {
    assert_eq!(
        search(tmpdir(), &["bin,obj@*.csproj,*.fsproj"]).unwrap(),
        vec![
            "dotnet-cs/bin",
            "dotnet-cs/obj",
            "dotnet-fs/bin",
            "dotnet-fs/obj"
        ]
    );
}

#[test]
fn mixed() {
    assert_eq!(
        search(tmpdir(), &["_build@rebar.config", "_build@mix.exs"]).unwrap(),
        vec!["mixed/_build"]
    );
}
