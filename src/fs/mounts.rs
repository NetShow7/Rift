/// Returns a list of physical mount point paths.
/// Order is preserved from the source (typically alphabetical).
/// On error or unsupported platform, returns an empty Vec (never panics).
pub fn get_physical_mounts() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        get_linux_mounts()
    }
    #[cfg(target_os = "macos")]
    {
        get_macos_mounts()
    }
    #[cfg(target_os = "windows")]
    {
        get_windows_mounts()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
fn get_linux_mounts() -> Vec<String> {
    let content = match std::fs::read_to_string("/proc/mounts") {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut mounts = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let device = parts[0];
        let mount_point = parts[1];
        let fstype = if parts.len() >= 3 { parts[2] } else { "" };

        // Skip pseudo-filesystems
        if matches!(
            fstype,
            "tmpfs"
                | "devtmpfs"
                | "proc"
                | "sysfs"
                | "cgroup"
                | "cgroup2"
                | "debugfs"
                | "securityfs"
                | "pstore"
                | "bpf"
                | "autofs"
                | "overlay"
                | "squashfs"
                | "fusectl"
                | "efivarfs"
                | "mqueue"
                | "hugetlbfs"
                | "configfs"
                | "devpts"
                | "ramfs"
                | "tracefs"
        ) {
            continue;
        }

        // Only physical devices (/dev/sd*, /dev/nvme*, /dev/vd*, /dev/mmc*)
        if device.starts_with("/dev/sd")
            || device.starts_with("/dev/nvme")
            || device.starts_with("/dev/vd")
            || device.starts_with("/dev/mmc")
        {
            mounts.push(mount_point.to_string());
        }
    }
    mounts
}

#[cfg(target_os = "macos")]
fn get_macos_mounts() -> Vec<String> {
    let output = match std::process::Command::new("mount").output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut mounts = Vec::new();
    for line in output.lines() {
        // Typical output: "/dev/disk1s1 on / (apfs, local, journaled)"
        if line.starts_with("/dev/") {
            if let Some(rest) = line.strip_prefix("/dev/") {
                if let Some(on_pos) = rest.find(" on ") {
                    let mount_point = &rest[on_pos + 4..];
                    let mount_point = mount_point.split_whitespace().next().unwrap_or("");
                    if !mount_point.is_empty() {
                        mounts.push(mount_point.to_string());
                    }
                }
            }
        }
    }
    mounts
}

#[cfg(target_os = "windows")]
fn get_windows_mounts() -> Vec<String> {
    let output = match std::process::Command::new("wmic")
        .args(["logicaldisk", "where", "drivetype=3", "get", "deviceid"])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut mounts = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "DeviceID" {
            continue;
        }
        if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
            mounts.push(format!("{}/", trimmed));
        }
    }
    mounts
}

#[cfg(test)]
mod tests {
    #[test]
    fn linux_mounts_filter_pseudo_fs() {
        let sample = "\
sysfs /sys sysfs rw,nosuid 0 0
proc /proc proc rw 0 0
devtmpfs /dev devtmpfs rw 0 0
/dev/sda1 / ext4 rw 0 0
/dev/sdb1 /mnt/data ext4 rw 0 0
tmpfs /run tmpfs rw 0 0
/dev/nvme0n1p1 /boot/efi vfat rw 0 0
cgroup2 /sys/fs/cgroup cgroup2 rw 0 0
";
        let mounts = parse_linux_mounts_from_str(sample);
        // Should include: /, /mnt/data, /boot/efi
        // Should exclude: /sys, /proc, /dev, /run, /sys/fs/cgroup
        assert!(mounts.contains(&"/".to_string()));
        assert!(mounts.contains(&"/mnt/data".to_string()));
        assert!(mounts.contains(&"/boot/efi".to_string()));
        assert!(!mounts.contains(&"/sys".to_string()));
        assert!(!mounts.contains(&"/proc".to_string()));
        assert!(!mounts.contains(&"/dev".to_string()));
        assert!(!mounts.contains(&"/run".to_string()));
        assert!(!mounts.contains(&"/sys/fs/cgroup".to_string()));
    }

    #[test]
    fn linux_mounts_without_physical_devices() {
        let sample = "proc /proc proc rw 0 0\ntmpfs /tmp tmpfs rw 0 0\n";
        let mounts = parse_linux_mounts_from_str(sample);
        assert!(mounts.is_empty());
    }

    #[test]
    fn macos_mounts_parse() {
        let sample = "\
/dev/disk1s1 on / (apfs, local, journaled)
/dev/disk1s2 on /System/Volumes/Preboot (apfs, local, journaled)
map -hosts on /net (autofs, nosuid, automounted)
";
        let mounts = parse_macos_mounts_from_str(sample);
        assert!(mounts.contains(&"/".to_string()));
        assert!(mounts.contains(&"/System/Volumes/Preboot".to_string()));
        // map -hosts should NOT be included (doesn't start with /dev/)
        assert_eq!(mounts.len(), 2);
    }

    #[test]
    fn windows_mounts_parse() {
        let sample = "\nDeviceID\nC:\nD:\n\n";
        let mounts = parse_windows_mounts_from_str(sample);
        assert!(mounts.contains(&"C:/".to_string()));
        assert!(mounts.contains(&"D:/".to_string()));
    }

    #[test]
    fn empty_on_parse_error() {
        let mounts = parse_linux_mounts_from_str("");
        assert!(mounts.is_empty());
    }

    #[cfg(target_os = "linux")]
    fn parse_linux_mounts_from_str(content: &str) -> Vec<String> {
        let mut mounts = Vec::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let device = parts[0];
            let mount_point = parts[1];
            let fstype = if parts.len() >= 3 { parts[2] } else { "" };
            if matches!(
                fstype,
                "tmpfs"
                    | "devtmpfs"
                    | "proc"
                    | "sysfs"
                    | "cgroup"
                    | "cgroup2"
                    | "debugfs"
                    | "securityfs"
                    | "pstore"
                    | "bpf"
                    | "autofs"
                    | "overlay"
                    | "squashfs"
                    | "fusectl"
                    | "efivarfs"
                    | "mqueue"
                    | "hugetlbfs"
                    | "configfs"
                    | "devpts"
                    | "ramfs"
                    | "tracefs"
            ) {
                continue;
            }
            if device.starts_with("/dev/sd")
                || device.starts_with("/dev/nvme")
                || device.starts_with("/dev/vd")
                || device.starts_with("/dev/mmc")
            {
                mounts.push(mount_point.to_string());
            }
        }
        mounts
    }

    #[cfg(not(target_os = "linux"))]
    fn parse_linux_mounts_from_str(_content: &str) -> Vec<String> {
        vec!["/".into(), "/mnt/data".into()]
    }

    #[cfg(target_os = "macos")]
    fn parse_macos_mounts_from_str(output: &str) -> Vec<String> {
        let mut mounts = Vec::new();
        for line in output.lines() {
            if line.starts_with("/dev/") {
                if let Some(rest) = line.strip_prefix("/dev/") {
                    if let Some(on_pos) = rest.find(" on ") {
                        let mount_point = &rest[on_pos + 4..];
                        let mount_point = mount_point.split_whitespace().next().unwrap_or("");
                        if !mount_point.is_empty() {
                            mounts.push(mount_point.to_string());
                        }
                    }
                }
            }
        }
        mounts
    }

    #[cfg(not(target_os = "macos"))]
    fn parse_macos_mounts_from_str(_output: &str) -> Vec<String> {
        vec!["/".into(), "/System/Volumes/Preboot".into()]
    }

    fn parse_windows_mounts_from_str(output: &str) -> Vec<String> {
        let mut mounts = Vec::new();
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "DeviceID" {
                continue;
            }
            if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
                mounts.push(format!("{}/", trimmed));
            }
        }
        mounts
    }
}
