use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

const REPORT_SIZE: usize = 64;
const REPORT_HEADER_SIZE: usize = 3;
const REPORT_PAYLOAD_SIZE: usize = REPORT_SIZE - REPORT_HEADER_SIZE;

const COMMAND_UPDATE: u8 = 0x01;
const TARGET_LEGACY_DISPLAY_TEXT: u8 = 0x01;
const TARGET_SLOT_1: u8 = 0x10;
const TARGET_SLOT_2: u8 = 0x11;
const TARGET_SLOT_3: u8 = 0x12;
const TARGET_SLOT_4: u8 = 0x13;

const MATCH_VENDOR_ID: &str = "2e8a";
const MATCH_PRODUCT_ID: &str = "00a0";
const MATCH_PRODUCT_NAME: &str = "Pico Inkpack Status";
const MATCH_INTERFACE_NAME: &str = "Status HID";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub dev_root: PathBuf,
    pub sys_hidraw_root: PathBuf,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self {
            dev_root: PathBuf::from("/dev"),
            sys_hidraw_root: PathBuf::from("/sys/class/hidraw"),
        }
    }
}

impl RuntimePaths {
    pub fn from_env() -> Self {
        let mut paths = Self::default();

        if let Some(value) = env::var_os("USB_STATE_SENDER_DEV_ROOT") {
            paths.dev_root = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("USB_STATE_SENDER_SYS_HIDRAW_ROOT") {
            paths.sys_hidraw_root = PathBuf::from(value);
        }

        paths
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TargetSlot {
    LegacyDisplayText,
    #[default]
    Slot1,
    Slot2,
    Slot3,
    Slot4,
}

impl TargetSlot {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "legacy" | "display" | "text" => Some(Self::LegacyDisplayText),
            "1" | "slot1" | "slot-1" => Some(Self::Slot1),
            "2" | "slot2" | "slot-2" => Some(Self::Slot2),
            "3" | "slot3" | "slot-3" => Some(Self::Slot3),
            "4" | "slot4" | "slot-4" => Some(Self::Slot4),
            _ => None,
        }
    }

    fn report_target(self) -> u8 {
        match self {
            Self::LegacyDisplayText => TARGET_LEGACY_DISPLAY_TEXT,
            Self::Slot1 => TARGET_SLOT_1,
            Self::Slot2 => TARGET_SLOT_2,
            Self::Slot3 => TARGET_SLOT_3,
            Self::Slot4 => TARGET_SLOT_4,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::LegacyDisplayText => "legacy",
            Self::Slot1 => "slot 1",
            Self::Slot2 => "slot 2",
            Self::Slot3 => "slot 3",
            Self::Slot4 => "slot 4",
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Usage(String),
    #[error("no matching Pico Inkpack Status HID device found under {0}. use --device PATH to select one explicitly")]
    DeviceNotFound(PathBuf),
    #[error("found multiple matching Pico Inkpack Status HID devices: {0}. use --device PATH to select one explicitly")]
    MultipleDevices(String),
    #[error("failed to enumerate {path}: {source}")]
    EnumeratePath { path: PathBuf, source: io::Error },
    #[error("failed to read {path}: {source}")]
    ReadPath { path: PathBuf, source: io::Error },
    #[error("failed to canonicalize {path}: {source}")]
    CanonicalizePath { path: PathBuf, source: io::Error },
    #[error("failed to write {path}: {source}")]
    WritePath { path: PathBuf, source: io::Error },
    #[error("failed to write command output: {0}")]
    Output(io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    List,
    Send {
        device: Option<PathBuf>,
        target: TargetSlot,
        message: String,
    },
}

pub fn run<I, S, W>(args: I, output: &mut W, paths: &RuntimePaths) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
{
    let parsed = parse_args(args)?;

    match parsed.command {
        Command::Help => output
            .write_all(usage(&parsed.program_name).as_bytes())
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
                None => select_single_device(paths)?,
            };
            let report = build_update_report(target, &message);
            write_report(&path, &report)?;
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

pub fn build_update_report(target: TargetSlot, message: &str) -> [u8; REPORT_SIZE] {
    let mut report = [0_u8; REPORT_SIZE];
    report[0] = COMMAND_UPDATE;
    report[1] = target.report_target();

    let mut payload_length = 0;
    for ch in message.chars() {
        if payload_length == REPORT_PAYLOAD_SIZE {
            break;
        }

        report[REPORT_HEADER_SIZE + payload_length] = sanitize_ascii(ch);
        payload_length += 1;
    }

    report[2] = payload_length as u8;
    report
}

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

fn parse_args<I, S>(args: I) -> Result<ParsedArgs, Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let program_name = args
        .next()
        .unwrap_or_else(|| String::from("usb-state-sender"));

    let mut device = None;
    let mut target = TargetSlot::default();
    let mut message_parts = Vec::new();
    let mut list = false;
    let mut positional_only = false;

    while let Some(arg) = args.next() {
        if !positional_only {
            match arg.as_str() {
                "-h" | "--help" => {
                    return Ok(ParsedArgs {
                        program_name,
                        command: Command::Help,
                    });
                }
                "--list" => {
                    list = true;
                    continue;
                }
                "--" => {
                    positional_only = true;
                    continue;
                }
                "--device" => {
                    let value = args.next().ok_or_else(|| {
                        Error::Usage(format!(
                            "missing value for --device\n\n{}",
                            usage(&program_name)
                        ))
                    })?;
                    device = Some(PathBuf::from(value));
                    continue;
                }
                "--slot" => {
                    let value = args.next().ok_or_else(|| {
                        Error::Usage(format!(
                            "missing value for --slot\n\n{}",
                            usage(&program_name)
                        ))
                    })?;
                    target = parse_target(&value, &program_name)?;
                    continue;
                }
                _ => {}
            }

            if let Some(value) = arg.strip_prefix("--device=") {
                device = Some(PathBuf::from(value));
                continue;
            }
            if let Some(value) = arg.strip_prefix("--slot=") {
                target = parse_target(value, &program_name)?;
                continue;
            }
            if arg.starts_with('-') {
                return Err(Error::Usage(format!(
                    "unknown option: {arg}\n\n{}",
                    usage(&program_name)
                )));
            }
        }

        message_parts.push(arg);
    }

    if list {
        if !message_parts.is_empty() {
            return Err(Error::Usage(format!(
                "--list does not accept a message\n\n{}",
                usage(&program_name)
            )));
        }
        return Ok(ParsedArgs {
            program_name,
            command: Command::List,
        });
    }

    if message_parts.is_empty() {
        return Err(Error::Usage(format!(
            "missing message text\n\n{}",
            usage(&program_name)
        )));
    }

    Ok(ParsedArgs {
        program_name,
        command: Command::Send {
            device,
            target,
            message: message_parts.join(" "),
        },
    })
}

fn parse_target(value: &str, program_name: &str) -> Result<TargetSlot, Error> {
    TargetSlot::parse(value).ok_or_else(|| {
        Error::Usage(format!(
            "invalid slot value: {value}\nexpected one of: legacy, 1, 2, 3, 4\n\n{}",
            usage(program_name)
        ))
    })
}

fn usage(program_name: &str) -> String {
    format!(
        "\
Usage:
  {program_name} [--device PATH] [--slot SLOT] <message...>
  {program_name} --list
  {program_name} --help

Options:
  --device PATH   Write to a specific hidraw device path.
  --slot SLOT     Select target slot: legacy, 1, 2, 3, or 4. Defaults to 1.
  --list          List auto-detected Pico Inkpack HID devices.
  --help          Show this help text.
"
    )
}

fn select_single_device(paths: &RuntimePaths) -> Result<PathBuf, Error> {
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

fn write_report(path: &Path, report: &[u8; REPORT_SIZE]) -> Result<(), Error> {
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

    let mut vendor_id = None;
    let mut product_id = None;
    let mut product_name = None;
    let mut interface_name = None;
    let mut hid_name = None;

    for ancestor in resolved.ancestors() {
        if vendor_id.is_none() {
            vendor_id = read_optional_trimmed(&ancestor.join("idVendor"))?;
        }
        if product_id.is_none() {
            product_id = read_optional_trimmed(&ancestor.join("idProduct"))?;
        }
        if product_name.is_none() {
            product_name = read_optional_trimmed(&ancestor.join("product"))?;
        }
        if interface_name.is_none() {
            interface_name = read_optional_trimmed(&ancestor.join("interface"))?;
        }
        if let Some(uevent) = read_optional_trimmed(&ancestor.join("uevent"))? {
            for line in uevent.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };

                match key {
                    "PRODUCT" if vendor_id.is_none() || product_id.is_none() => {
                        let mut parts = value.split('/');
                        if vendor_id.is_none() {
                            vendor_id = parts.next().and_then(parse_hex_component);
                        }
                        if product_id.is_none() {
                            product_id = parts.next().and_then(parse_hex_component);
                        }
                    }
                    "HID_NAME" if hid_name.is_none() => {
                        hid_name = Some(value.trim().to_owned());
                    }
                    _ => {}
                }
            }
        }
    }

    let vendor_matches = vendor_id
        .as_deref()
        .map(normalize_hex_id)
        .is_some_and(|value| value == MATCH_VENDOR_ID);
    let product_matches = product_id
        .as_deref()
        .map(normalize_hex_id)
        .is_some_and(|value| value == MATCH_PRODUCT_ID);
    let name_matches = product_name.as_deref() == Some(MATCH_PRODUCT_NAME);
    let interface_matches = interface_name.as_deref() == Some(MATCH_INTERFACE_NAME)
        || hid_name.as_deref() == Some(MATCH_INTERFACE_NAME);

    Ok(vendor_matches && product_matches && (name_matches || interface_matches))
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

fn sanitize_ascii(ch: char) -> u8 {
    match ch {
        '\n' | '\r' | '\t' => ch as u8,
        ' '..='~' => ch as u8,
        _ if ch.is_ascii() => b'?',
        _ => b'?',
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedArgs {
    program_name: String,
    command: Command,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_update_report_sets_expected_header() {
        let report = build_update_report(TargetSlot::Slot2, "IP: 192.0.2.10");

        assert_eq!(report[0], COMMAND_UPDATE);
        assert_eq!(report[1], TARGET_SLOT_2);
        assert_eq!(report[2], 14);
        assert_eq!(&report[3..17], b"IP: 192.0.2.10");
        assert!(report[17..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn build_update_report_truncates_and_replaces_non_ascii() {
        let message = format!("abc{}", "あ".repeat(70));
        let report = build_update_report(TargetSlot::Slot1, &message);

        assert_eq!(report[2] as usize, REPORT_PAYLOAD_SIZE);
        assert_eq!(&report[3..7], b"abc?");
    }

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
