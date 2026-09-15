use crate::DiscoveredDevice;
use forgehx_core::DiagnosticReport;

pub fn doctor_reports(devices: &[DiscoveredDevice]) -> Vec<DiagnosticReport> {
    let mut reports = Vec::new();
    for device in devices {
        for raw in &device.raw_interfaces {
            reports.push(DiagnosticReport {
                device_id: device.info.id.clone(),
                source: raw.source,
                path: raw.path.clone(),
                vendor_id: (raw.vendor_id != 0).then_some(raw.vendor_id),
                product_id: (raw.product_id != 0).then_some(raw.product_id),
                manufacturer: raw.manufacturer.clone(),
                product: raw.product.clone(),
                serial: raw.serial.clone(),
                interface_number: raw.interface_number,
                usage_page: raw.usage_page,
                usage: raw.usage,
                audio_node_id: raw.audio_node_id,
                usb_parent: raw.usb_parent.clone(),
                protocol_match: device.info.protocol.clone(),
                support_level: device.info.support_level,
                write_protected: device.info.protocol.is_none(),
            });
        }
    }
    reports.sort_by(|a, b| a.device_id.0.cmp(&b.device_id.0).then(a.path.cmp(&b.path)));
    reports
}
