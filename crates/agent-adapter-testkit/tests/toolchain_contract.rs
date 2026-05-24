use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn pinned_toolchain_provisions_documented_verification_tools() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config: toml::Value = std::fs::read_to_string(root.join("rust-toolchain.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let components: BTreeSet<_> = config["toolchain"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();

    for document in ["docs/build.md", "docs/ci.md", ".github/workflows/ci.yml"] {
        let contents = std::fs::read_to_string(root.join(document)).unwrap();
        for (command, component) in [("cargo fmt", "rustfmt"), ("cargo clippy", "clippy")] {
            if contents.contains(command) {
                assert!(
                    components.contains(component),
                    "{document} invokes {command}, but the pinned toolchain does not install {component}"
                );
            }
        }
    }
}
