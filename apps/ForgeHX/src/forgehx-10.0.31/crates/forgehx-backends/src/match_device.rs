use forgehx_core::DeviceInfo;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIdentity {
    pub id: String,
    pub name: String,
    pub vendor: Option<String>,
    pub serial: Option<String>,
    pub location: Option<String>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MatchError {
    #[error("no deterministic external-device match")]
    NoMatch,
    #[error("ambiguous external-device match")]
    Ambiguous,
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn score(device: &DeviceInfo, external: &ExternalIdentity) -> i32 {
    let mut score = 0;
    if let (Some(a), Some(b)) = (device.serial.as_deref(), external.serial.as_deref()) {
        if !a.is_empty() && a.eq_ignore_ascii_case(b) {
            score += 120;
        }
    }
    if external.vendor_id == Some(device.vendor_id)
        && external.product_id == Some(device.product_id)
        && device.vendor_id != 0
    {
        score += 100;
    }
    if let Some(location) = &external.location {
        let needle = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);
        if device.vendor_id != 0 && location.to_ascii_lowercase().contains(&needle) {
            score += 80;
        }
    }
    if let (Some(manufacturer), Some(vendor)) =
        (device.manufacturer.as_deref(), external.vendor.as_deref())
    {
        let a = normalize(manufacturer);
        let b = normalize(vendor);
        if !a.is_empty() && !b.is_empty() && (a.contains(&b) || b.contains(&a)) {
            score += 25;
        }
    }
    let a = normalize(&device.name);
    let b = normalize(&external.name);
    if !a.is_empty() && !b.is_empty() && (a == b || a.contains(&b) || b.contains(&a)) {
        score += 50;
    }
    score
}

pub fn match_external_device<'a>(
    device: &DeviceInfo,
    external: &'a [ExternalIdentity],
) -> Result<&'a ExternalIdentity, MatchError> {
    let mut scored = external
        .iter()
        .map(|item| (score(device, item), item))
        .filter(|(score, _)| *score >= 50)
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
    let Some((best_score, best)) = scored.first().copied() else {
        return Err(MatchError::NoMatch);
    };
    if scored.get(1).is_some_and(|(score, _)| *score == best_score) {
        return Err(MatchError::Ambiguous);
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{ConnectionKind, DeviceClass, DeviceId, SupportLevel, VendorFamily};

    fn device() -> DeviceInfo {
        DeviceInfo {
            id: DeviceId("hx".into()),
            name: "HyperX Alloy Origins".into(),
            manufacturer: Some("HyperX".into()),
            vendor_id: 0x0951,
            product_id: 0x16e5,
            serial: Some("ABC123".into()),
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Keyboard,
            support_level: SupportLevel::DiagnosticOnly,
            connection_kind: ConnectionKind::Hid,
            interfaces: vec![],
            capabilities: vec![],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: None,
        }
    }

    #[test]
    fn serial_exact_match_wins() {
        let values = vec![
            ExternalIdentity {
                id: "0".into(),
                name: "HyperX Alloy Origins".into(),
                vendor: Some("HyperX".into()),
                serial: None,
                location: None,
                vendor_id: None,
                product_id: None,
            },
            ExternalIdentity {
                id: "1".into(),
                name: "Keyboard".into(),
                vendor: Some("HyperX".into()),
                serial: Some("ABC123".into()),
                location: None,
                vendor_id: None,
                product_id: None,
            },
        ];
        assert_eq!(match_external_device(&device(), &values).unwrap().id, "1");
    }
}
