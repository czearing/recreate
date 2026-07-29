use super::*;

#[test]
fn asset_paths_cannot_escape_build_root() {
    let root = Path::new("dist");
    assert_eq!(asset_path(root, "/../secret"), root.join("secret"));
    assert_eq!(asset_path(root, "/"), root.join("index.html"));
}
