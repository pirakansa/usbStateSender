mod cli;
mod device;
mod error;
mod paths;
mod report;

use std::io::Write;

use cli::Command;

pub use device::list_matching_devices;
pub use error::Error;
pub use paths::RuntimePaths;
pub use report::{build_update_report, TargetSlot};

pub fn run<I, S, W>(args: I, output: &mut W, paths: &RuntimePaths) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
{
    let parsed = cli::parse_args(args)?;

    match parsed.command {
        Command::Help => output
            .write_all(cli::usage(&parsed.program_name).as_bytes())
            .map_err(Error::Output),
        Command::List => {
            for device in list_matching_devices(paths)? {
                writeln!(output, "{}", device.display()).map_err(Error::Output)?;
            }
            Ok(())
        }
        Command::Send {
            device,
            target,
            message,
        } => {
            let path = match device {
                Some(path) => path,
                None => device::select_single_device(paths)?,
            };
            let report = build_update_report(target, &message);
            device::write_report(&path, &report)?;
            writeln!(
                output,
                "sent {} update to {}",
                target.label(),
                path.display()
            )
            .map_err(Error::Output)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::device::{MATCH_PRODUCT_ID, MATCH_VENDOR_ID};

    #[test]
    fn run_lists_devices() {
        let temp = tempdir().expect("tempdir should be created");
        let dev_root = temp.path().join("dev");
        let sys_root = temp.path().join("sys/class/hidraw");
        fs::create_dir_all(&dev_root).expect("dev root should be created");
        fs::create_dir_all(&sys_root).expect("sys root should be created");
        create_fake_hidraw(
            &dev_root,
            &sys_root,
            "hidraw3",
            MATCH_VENDOR_ID,
            MATCH_PRODUCT_ID,
        );

        let mut output = Vec::new();
        run(
            ["usb-state-sender", "--list"],
            &mut output,
            &RuntimePaths {
                dev_root,
                sys_hidraw_root: sys_root,
            },
        )
        .expect("listing should succeed");

        let output = String::from_utf8(output).expect("output should be utf-8");
        assert!(output.contains("hidraw3"));
    }

    fn create_fake_hidraw(
        dev_root: &Path,
        sys_root: &Path,
        name: &str,
        vendor_id: &str,
        product_id: &str,
    ) {
        fs::write(dev_root.join(name), []).expect("device file should be created");

        let device_dir = sys_root.join(name).join("device");
        fs::create_dir_all(&device_dir).expect("device sysfs directory should be created");
        fs::write(device_dir.join("idVendor"), format!("{vendor_id}\n"))
            .expect("vendor id should be written");
        fs::write(device_dir.join("idProduct"), format!("{product_id}\n"))
            .expect("product id should be written");
        fs::write(
            device_dir.join("product"),
            format!("{}\n", crate::device::MATCH_PRODUCT_NAME),
        )
        .expect("product name should be written");
        fs::write(
            device_dir.join("interface"),
            format!("{}\n", crate::device::MATCH_INTERFACE_NAME),
        )
        .expect("interface name should be written");
    }
}
