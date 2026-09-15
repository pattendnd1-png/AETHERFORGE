#![forbid(unsafe_code)]
//! M0 shader profile contracts and structural WGSL preflight validation.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderProfile {
    pub id: String,
    pub display_name: String,
    pub source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShaderValidationError {
    EmptySource,
    UnbalancedBraces,
}

pub fn validate_wgsl_source(source: &str) -> Result<(), ShaderValidationError> {
    if source.trim().is_empty() {
        return Err(ShaderValidationError::EmptySource);
    }

    let mut depth = 0_u32;
    for character in source.chars() {
        match character {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                if depth == 0 {
                    return Err(ShaderValidationError::UnbalancedBraces);
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    if depth == 0 {
        Ok(())
    } else {
        Err(ShaderValidationError::UnbalancedBraces)
    }
}
