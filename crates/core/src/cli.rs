//! Command-line parsing for the `--device` launch option.

/// Extracts the validated `--device <path>` / `--device=<path>` value.
///
/// Only plain device nodes directly under `/dev/` are accepted (`/dev/sda`,
/// `/dev/sda1`, `/dev/nvme0n1p1`): anything else could point somewhere unexpected.
///
/// The error is the rejected value (empty when none was given); turn it into a
/// message with `view::device_arg_error_message`.
pub fn parse_device_arg(args: &[String]) -> Result<Option<String>, String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let value = if arg == "--device" {
            iter.next().map(String::as_str).unwrap_or("")
        } else if let Some(inline) = arg.strip_prefix("--device=") {
            inline
        } else {
            continue;
        };
        return validate_device(value).map(Some);
    }
    Ok(None)
}

fn validate_device(value: &str) -> Result<String, String> {
    let name = value
        .strip_prefix("/dev/")
        .ok_or_else(|| value.to_owned())?;
    let plain = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if plain && !name.contains("..") {
        Ok(value.to_owned())
    } else {
        Err(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn no_device_option_means_the_normal_disk_list() {
        assert_eq!(parse_device_arg(&args(&["kformat"])), Ok(None));
        assert_eq!(parse_device_arg(&args(&[])), Ok(None));
    }

    #[test]
    fn separate_and_equals_forms_are_both_accepted() {
        assert_eq!(
            parse_device_arg(&args(&["kformat", "--device", "/dev/sda"])),
            Ok(Some("/dev/sda".to_owned()))
        );
        assert_eq!(
            parse_device_arg(&args(&["kformat", "--device=/dev/sda1"])),
            Ok(Some("/dev/sda1".to_owned()))
        );
    }

    #[test]
    fn common_device_node_names_are_accepted() {
        for node in [
            "/dev/sdb",
            "/dev/nvme0n1p2",
            "/dev/mmcblk0p1",
            "/dev/vda3",
            "/dev/loop8",
        ] {
            assert_eq!(
                parse_device_arg(&args(&["k", "--device", node])),
                Ok(Some(node.to_owned())),
                "{node}"
            );
        }
    }

    #[test]
    fn other_arguments_are_ignored() {
        let given = args(&["kformat", "-platform", "offscreen", "--device", "/dev/sda"]);
        assert_eq!(parse_device_arg(&given), Ok(Some("/dev/sda".to_owned())));
    }

    #[test]
    fn a_missing_value_is_an_error() {
        assert!(parse_device_arg(&args(&["kformat", "--device"])).is_err());
        assert!(parse_device_arg(&args(&["kformat", "--device="])).is_err());
    }

    #[test]
    fn paths_outside_plain_dev_nodes_are_rejected() {
        for bad in [
            "sda",
            "/dev/",
            "/dev/../etc/passwd",
            "/dev/mapper/root",
            "/etc/passwd",
            "/dev/sda;rm",
            "/dev/sd a",
            "/dev/.hidden",
        ] {
            assert!(
                parse_device_arg(&args(&["k", "--device", bad])).is_err(),
                "{bad} must be rejected"
            );
        }
    }
}
