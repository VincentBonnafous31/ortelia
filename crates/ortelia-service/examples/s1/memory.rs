//! Memory of the process, read in `/proc/self/status` under Linux; not available elsewhere.

/// Peak resident memory of the process (`VmHWM`), in MiB.
pub fn peak_mib() -> Option<f64> {
    status_kib("VmHWM:").map(kib_to_mib)
}

/// Current resident memory of the process (`VmRSS`), in MiB.
pub fn resident_mib() -> Option<f64> {
    status_kib("VmRSS:").map(kib_to_mib)
}

fn kib_to_mib(kib: u64) -> f64 {
    kib as f64 / 1024.0
}

/// Reads a field of `/proc/self/status`, such as `VmHWM:   123456 kB`, in KiB.
#[cfg(target_os = "linux")]
fn status_kib(field: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    parse_status(&status, field)
}

#[cfg(not(target_os = "linux"))]
fn status_kib(_field: &str) -> Option<u64> {
    None
}

/// Finds `field` in the text of `/proc/self/status` and parses its value in KiB.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_status(status: &str, field: &str) -> Option<u64> {
    let line = status.lines().find(|line| line.starts_with(field))?;
    line[field.len()..]
        .trim()
        .strip_suffix("kB")?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_status_field() {
        let status = "Name:\ts1\nVmPeak:\t  20000 kB\nVmHWM:\t   1536 kB\nVmRSS:\t   1024 kB\n";
        assert_eq!(parse_status(status, "VmHWM:"), Some(1536));
        assert_eq!(parse_status(status, "VmRSS:"), Some(1024));
        assert_eq!(parse_status(status, "VmSwap:"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reads_the_memory_of_this_process() {
        let peak = peak_mib().expect("VmHWM");
        let resident = resident_mib().expect("VmRSS");
        assert!(peak > 0.0 && resident > 0.0 && peak >= resident * 0.5);
    }
}
