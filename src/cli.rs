use std::path::PathBuf;

use crate::error::Error;
use crate::report::TargetSlot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    List,
    Send {
        device: Option<PathBuf>,
        target: TargetSlot,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedArgs {
    pub(crate) program_name: String,
    pub(crate) command: Command,
}

pub(crate) fn parse_args<I, S>(args: I) -> Result<ParsedArgs, Error>
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
            if let Some(parsed) = parse_flag(&arg, &mut args, &program_name)? {
                match parsed {
                    ParsedFlag::Help => {
                        return Ok(ParsedArgs {
                            program_name,
                            command: Command::Help,
                        });
                    }
                    ParsedFlag::List => list = true,
                    ParsedFlag::PositionalOnly => positional_only = true,
                    ParsedFlag::Device(path) => device = Some(path),
                    ParsedFlag::Target(slot) => target = slot,
                }
                continue;
            }
        }

        message_parts.push(arg);
    }

    command_from_parts(program_name, list, device, target, message_parts)
}

pub(crate) fn usage(program_name: &str) -> String {
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

enum ParsedFlag {
    Help,
    List,
    PositionalOnly,
    Device(PathBuf),
    Target(TargetSlot),
}

fn parse_flag(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    program_name: &str,
) -> Result<Option<ParsedFlag>, Error> {
    match arg {
        "-h" | "--help" => return Ok(Some(ParsedFlag::Help)),
        "--list" => return Ok(Some(ParsedFlag::List)),
        "--" => return Ok(Some(ParsedFlag::PositionalOnly)),
        "--device" => {
            let value = next_option_value(args, "--device", program_name)?;
            return Ok(Some(ParsedFlag::Device(PathBuf::from(value))));
        }
        "--slot" => {
            let value = next_option_value(args, "--slot", program_name)?;
            return Ok(Some(ParsedFlag::Target(parse_target(
                &value,
                program_name,
            )?)));
        }
        _ => {}
    }

    if let Some(value) = arg.strip_prefix("--device=") {
        return Ok(Some(ParsedFlag::Device(PathBuf::from(value))));
    }
    if let Some(value) = arg.strip_prefix("--slot=") {
        return Ok(Some(ParsedFlag::Target(parse_target(value, program_name)?)));
    }
    if arg.starts_with('-') {
        return Err(Error::Usage(format!(
            "unknown option: {arg}\n\n{}",
            usage(program_name)
        )));
    }

    Ok(None)
}

fn next_option_value(
    args: &mut impl Iterator<Item = String>,
    option: &str,
    program_name: &str,
) -> Result<String, Error> {
    args.next().ok_or_else(|| {
        Error::Usage(format!(
            "missing value for {option}\n\n{}",
            usage(program_name)
        ))
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

fn command_from_parts(
    program_name: String,
    list: bool,
    device: Option<PathBuf>,
    target: TargetSlot,
    message_parts: Vec<String>,
) -> Result<ParsedArgs, Error> {
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
