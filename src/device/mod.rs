mod descriptors;

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use descriptors::HidDescriptors;

use crate::error::Error;
use crate::paths::RuntimePaths;
use crate::report::REPORT_SIZE;

pub(crate) const MATCH_VENDOR_ID: &str = "2e8a";
pub(crate) const MATCH_PRODUCT_ID: &str = "00a0";
pub(crate) const MATCH_PRODUCT_NAME: &str = "Pico Inkpack Status";
pub(crate) const MATCH_INTERFACE_NAME: &str = "Status HID";

pub fn list_matching_devices(paths: &RuntimePaths) -> Result<Vec<PathBuf>, Error> {
    let entries = fs::read_dir(&paths.sys_hidraw_root).map_err(|source| Error::EnumeratePath {
        path: paths.sys_hidraw_root.clone(),
        source,
    })?;

    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::EnumeratePath {
            path: paths.sys_hidraw_root.clone(),
            source,
        })?;
        let device_name = entry.file_name().to_string_lossy().into_owned();
        if !device_name.starts_with("hidraw") {
            continue;
        }

        let sys_device_path = entry.path().join("device");
        if hidraw_device_matches(&sys_device_path)? {
            matches.push(paths.dev_root.join(device_name));
        }
    }

    matches.sort();
    Ok(matches)
}

pub(crate) fn select_single_device(paths: &RuntimePaths) -> Result<PathBuf, Error> {
    let matches = list_matching_devices(paths)?;
    match matches.len() {
        0 => Err(Error::DeviceNotFound(paths.sys_hidraw_root.clone())),
        1 => Ok(matches.into_iter().next().expect("len checked")),
        _ => Err(Error::MultipleDevices(
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

pub(crate) fn write_report(path: &Path, report: &[u8; REPORT_SIZE]) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|source| Error::WritePath {
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(report).map_err(|source| Error::WritePath {
        path: path.to_path_buf(),
        source,
    })
}

fn hidraw_device_matches(sys_device_path: &Path) -> Result<bool, Error> {
    let resolved = match fs::canonicalize(sys_device_path) {
        Ok(path) => path,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(Error::CanonicalizePath {
                path: sys_device_path.to_path_buf(),
                source,
            });
        }
    };

    let descriptors = HidDescriptors::read_from_ancestors(&resolved)?;

    Ok(descriptors.vendor_matches()
        && descriptors.product_matches()
        && descriptors.name_or_interface_matches())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn list_matching_devices_only_returns_matching_hidraw_entries() {
        let temp = tempdir().expect("tempdir should be created");
        let dev_root = temp.path().join("dev");
        let sys_root = temp.path().join("sys/class/hidraw");
        fs::create_dir_all(&dev_root).expect("dev root should be created");
        fs::create_dir_all(&sys_root).expect("sys root should be created");

        create_fake_hidraw(
            &dev_root,
            &sys_root,
            "hidraw7",
            MATCH_VENDOR_ID,
            MATCH_PRODUCT_ID,
        );
        create_fake_hidraw(&dev_root, &sys_root, "hidraw8", "1234", MATCH_PRODUCT_ID);

        let matches = list_matching_devices(&RuntimePaths {
            dev_root,
            sys_hidraw_root: sys_root,
        })
        .expect("device listing should succeed");

        assert_eq!(matches.len(), 1);
        assert!(matches[0].ends_with("hidraw7"));
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
            format!("{MATCH_PRODUCT_NAME}\n"),
        )
        .expect("product name should be written");
        fs::write(
            device_dir.join("interface"),
            format!("{MATCH_INTERFACE_NAME}\n"),
        )
        .expect("interface name should be written");
    }
}
