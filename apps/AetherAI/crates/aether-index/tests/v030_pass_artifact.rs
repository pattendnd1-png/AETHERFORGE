use std::path::Path;

use aether_index::{PassCheckpoint, parse_pass_checkpoint};

#[test]
fn versioned_verify_file_produces_checkpoint() {
    let text = concat!(
        "AETHERAI_VERSION=0.2.2\n",
        "AETHERAI_PLATFORM=linux-x86_64\n",
        "AETHERAI_BUILD=PASS\n",
        "AETHERAI_ARCHIVE_SHA256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        "AETHERAI_V0_2_2_VERIFY=PASS\n",
    );

    let checkpoint = parse_pass_checkpoint(Path::new("AetherAI-v0.2.2-VERIFY.txt"), text).unwrap();

    assert_eq!(checkpoint.version.as_deref(), Some("0.2.2"));
    assert_eq!(checkpoint.platform.as_deref(), Some("linux-x86_64"));
    assert_eq!(checkpoint.pass_endpoint, "AETHERAI_V0_2_2_VERIFY=PASS");
    assert_eq!(
        checkpoint.referenced_checksums,
        vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]
    );
}

#[test]
fn readme_and_markdown_fence_examples_do_not_produce_checkpoints() {
    let readme =
        "Expected endpoint:\n```text\nAETHERAI_VERSION=0.3.0\nAETHERAI_V0_3_0_VERIFY=PASS\n```\n";
    assert!(parse_pass_checkpoint(Path::new("README.md"), readme).is_none());

    let verify = "```text\nAETHERAI_VERSION=0.3.0\nAETHERAI_V0_3_0_VERIFY=PASS\n```\n";
    assert!(parse_pass_checkpoint(Path::new("AetherAI-v0.3.0-VERIFY.txt"), verify).is_none());
}

#[test]
fn prose_prefixed_pass_is_rejected_but_package_verify_is_eligible() {
    let bad = "AETHERAI_VERSION=0.3.0\nExpected: AETHERAI_V0_3_0_VERIFY=PASS\n";
    assert!(parse_pass_checkpoint(Path::new("AetherAI-v0.3.0-VERIFY.txt"), bad).is_none());

    let package = "AETHERAI_VERSION=0.3.0\nAETHERAI_V0_3_0_PACKAGE_VERIFY=PASS\n";
    let checkpoint =
        parse_pass_checkpoint(Path::new("AetherAI-v0.3.0-PACKAGE-VERIFY(1).txt"), package).unwrap();
    assert_eq!(
        checkpoint.pass_endpoint,
        "AETHERAI_V0_3_0_PACKAGE_VERIFY=PASS"
    );
}

#[test]
fn checkpoint_type_is_stable_and_small() {
    let checkpoint = PassCheckpoint {
        version: Some("0.3.0".into()),
        pass_endpoint: "AETHERAI_V0_3_0_VERIFY=PASS".into(),
        platform: None,
        referenced_checksums: Vec::new(),
    };
    assert_eq!(checkpoint.version.as_deref(), Some("0.3.0"));
}
