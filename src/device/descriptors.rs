use std::fs;
use std::io;
use std::path::Path;

use super::{MATCH_INTERFACE_NAME, MATCH_PRODUCT_ID, MATCH_PRODUCT_NAME, MATCH_VENDOR_ID};
use crate::error::Error;

#[derive(Debug, Default)]
pub(super) struct HidDescriptors {
    vendor_id: Option<String>,
    product_id: Option<String>,
    product_name: Option<String>,
    interface_name: Option<String>,
    hid_name: Option<String>,
}

impl HidDescriptors {
    pub(super) fn read_from_ancestors(path: &Path) -> Result<Self, Error> {
        let mut descriptors = Self::default();

        for ancestor in path.ancestors() {
            descriptors.read_files_from(ancestor)?;
            descriptors.read_uevent_from(ancestor)?;
        }

        Ok(descriptors)
    }

    fn read_files_from(&mut self, ancestor: &Path) -> Result<(), Error> {
        if self.vendor_id.is_none() {
            self.vendor_id = read_optional_trimmed(&ancestor.join("idVendor"))?;
        }
        if self.product_id.is_none() {
            self.product_id = read_optional_trimmed(&ancestor.join("idProduct"))?;
        }
        if self.product_name.is_none() {
            self.product_name = read_optional_trimmed(&ancestor.join("product"))?;
        }
        if self.interface_name.is_none() {
            self.interface_name = read_optional_trimmed(&ancestor.join("interface"))?;
        }

        Ok(())
    }

    fn read_uevent_from(&mut self, ancestor: &Path) -> Result<(), Error> {
        let Some(uevent) = read_optional_trimmed(&ancestor.join("uevent"))? else {
            return Ok(());
        };

        for line in uevent.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key {
                "PRODUCT" if self.vendor_id.is_none() || self.product_id.is_none() => {
                    self.read_product_uevent(value);
                }
                "HID_NAME" if self.hid_name.is_none() => {
                    self.hid_name = Some(value.trim().to_owned());
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn read_product_uevent(&mut self, value: &str) {
        let mut parts = value.split('/');
        if self.vendor_id.is_none() {
            self.vendor_id = parts.next().and_then(parse_hex_component);
        }
        if self.product_id.is_none() {
            self.product_id = parts.next().and_then(parse_hex_component);
        }
    }

    pub(super) fn vendor_matches(&self) -> bool {
        self.vendor_id
            .as_deref()
            .map(normalize_hex_id)
            .is_some_and(|value| value == MATCH_VENDOR_ID)
    }

    pub(super) fn product_matches(&self) -> bool {
        self.product_id
            .as_deref()
            .map(normalize_hex_id)
            .is_some_and(|value| value == MATCH_PRODUCT_ID)
    }

    pub(super) fn name_or_interface_matches(&self) -> bool {
        self.product_name.as_deref() == Some(MATCH_PRODUCT_NAME)
            || self.interface_name.as_deref() == Some(MATCH_INTERFACE_NAME)
            || self.hid_name.as_deref() == Some(MATCH_INTERFACE_NAME)
    }
}

fn read_optional_trimmed(path: &Path) -> Result<Option<String>, Error> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content.trim().to_owned())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::ReadPath {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn parse_hex_component(value: &str) -> Option<String> {
    u16::from_str_radix(value.trim(), 16)
        .ok()
        .map(|parsed| format!("{parsed:04x}"))
}

fn normalize_hex_id(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("0x")
        .chars()
        .all(|ch| ch.is_ascii_hexdigit())
        .then_some(value.trim().trim_start_matches("0x"))
        .and_then(parse_hex_component)
        .unwrap_or_else(|| value.trim().to_ascii_lowercase())
}
