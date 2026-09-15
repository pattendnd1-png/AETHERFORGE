use forgehx_core::Capability;
use forgehx_mouse::registry::MOUSE_MODELS;

#[derive(Debug, Clone)]
pub struct DriverDefinition {
    pub id: &'static str,
    pub capabilities: &'static [Capability],
    pub complete: bool,
}

// Discovery promotion is restricted to exact VID/PID identities that the shared
// mouse registry marks with an implemented native driver. Alias-only model matches
// never reach this function and therefore never gain raw-write authority.
pub fn driver_for(vendor_id: u16, product_id: u16) -> Option<DriverDefinition> {
    let model = MOUSE_MODELS.iter().find(|model| {
        model.native_driver.is_some() && model.exact_ids.contains(&(vendor_id, product_id))
    })?;
    Some(DriverDefinition {
        id: model.native_driver.expect("native model has driver id"),
        capabilities: model.native_capabilities,
        complete: model.complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_native_ids_are_registered() {
        assert_eq!(
            driver_for(0x03f0, 0x028e).unwrap().id,
            "hyperx-pulsefire-haste-wireless-v1"
        );
        assert_eq!(
            driver_for(0x03f0, 0x04bf).unwrap().id,
            "hyperx-pulsefire-saga-pro-v1"
        );
    }

    #[test]
    fn unrelated_hp_device_is_not_promoted() {
        assert!(driver_for(0x03f0, 0xffff).is_none());
    }
}
