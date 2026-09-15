use aether_shaders::{ShaderValidationError, validate_wgsl_source};

#[test]
fn empty_shader_source_is_rejected() {
    assert_eq!(
        validate_wgsl_source("   \n"),
        Err(ShaderValidationError::EmptySource)
    );
}

#[test]
fn unbalanced_shader_braces_are_rejected() {
    assert_eq!(
        validate_wgsl_source("@fragment fn main() {"),
        Err(ShaderValidationError::UnbalancedBraces)
    );
}

#[test]
fn structurally_balanced_shader_passes_m0_preflight() {
    assert!(validate_wgsl_source("@fragment fn main() { let x = 1; }").is_ok());
}
