use dbus_udisks2::DiskDevice;
use std::env;
use std::path::Path;

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

/// Whether the path points to an existing `.iso` or `.img` file.
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("iso") || ext.eq_ignore_ascii_case("img"))
        && path.exists()
}

/// If running in pkexec or sudo, restore the home directory for the open dialog,
/// and then downgrade permissions back to a regular user.
pub fn downgrade_from_pkexec() {
    if let Ok(pkexec_uid) = env::var("PKEXEC_UID").or_else(|_| env::var("SUDO_UID"))
        && let Ok(uid) = pkexec_uid.parse::<u32>()
        && let Some(passwd) = pwd::Passwd::from_uid(uid)
    {
        // SAFETY: the process is still single-threaded at this point.
        unsafe {
            env::set_var("HOME", passwd.dir);
            libc::setresgid(passwd.gid, passwd.gid, passwd.gid);
            libc::setresuid(passwd.uid, passwd.uid, passwd.uid);
        }
    }
}
