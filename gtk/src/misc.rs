use dbus_udisks2::DiskDevice;

pub fn device_label(device: &DiskDevice) -> String {
    if device.drive.vendor.is_empty() {
        format!("{} ({})", device.drive.model, device.parent.preferred_device.display())
    } else {
        format!(
            "{} {} ({})",
            device.drive.vendor,
            device.drive.model,
            device.parent.preferred_device.display()
        )
    }
}
