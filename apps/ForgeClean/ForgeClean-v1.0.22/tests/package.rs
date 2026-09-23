use forgeclean::package::{
    PackageArchive, VersionComparator, parse_package_filename, select_superseded,
};
use std::path::Path;

struct NumericCmp;
impl VersionComparator for NumericCmp {
    fn compare(&self, a: &str, b: &str) -> Result<std::cmp::Ordering, String> {
        let an = a.parse::<u32>().map_err(|e| e.to_string())?;
        let bn = b.parse::<u32>().map_err(|e| e.to_string())?;
        Ok(an.cmp(&bn))
    }
}

#[test]
fn parses_hyphenated_package_name_from_right() {
    let p = parse_package_filename(Path::new("linux-zen-6-3-x86_64.pkg.tar.zst")).unwrap();
    assert_eq!(p.name, "linux-zen");
    assert_eq!(p.version, "6-3");
    assert_eq!(p.arch, "x86_64");
}

#[test]
fn rejects_non_package_file() {
    assert!(parse_package_filename(Path::new("notes.txt")).is_none());
}

#[test]
fn retention_keeps_two_newest_versions() {
    let mk = |version: &str| PackageArchive::for_test("mesa", version, "x86_64");
    let old = select_superseded(vec![mk("1"), mk("3"), mk("2")], 2, &NumericCmp).unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].version, "1");
}

#[test]
fn retention_never_accepts_zero_keep() {
    let result = select_superseded(
        vec![PackageArchive::for_test("mesa", "1", "x86_64")],
        0,
        &NumericCmp,
    );
    assert!(result.is_err());
}
