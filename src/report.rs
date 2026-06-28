pub const REPORT_SIZE: usize = 64;
const REPORT_HEADER_SIZE: usize = 3;
const REPORT_PAYLOAD_SIZE: usize = REPORT_SIZE - REPORT_HEADER_SIZE;

const COMMAND_UPDATE: u8 = 0x01;
const TARGET_LEGACY_DISPLAY_TEXT: u8 = 0x01;
const TARGET_SLOT_1: u8 = 0x10;
const TARGET_SLOT_2: u8 = 0x11;
const TARGET_SLOT_3: u8 = 0x12;
const TARGET_SLOT_4: u8 = 0x13;

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
    pub(crate) fn parse(value: &str) -> Option<Self> {
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

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::LegacyDisplayText => "legacy",
            Self::Slot1 => "slot 1",
            Self::Slot2 => "slot 2",
            Self::Slot3 => "slot 3",
            Self::Slot4 => "slot 4",
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

fn sanitize_ascii(ch: char) -> u8 {
    match ch {
        '\n' | '\r' | '\t' => ch as u8,
        ' '..='~' => ch as u8,
        _ if ch.is_ascii() => b'?',
        _ => b'?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
