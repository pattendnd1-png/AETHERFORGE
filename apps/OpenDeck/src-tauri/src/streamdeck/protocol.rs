pub const ELGATO_VID: u16 = 0x0fd9;
pub const STREAM_DECK_PLUS_PID: u16 = 0x0084;
pub const REPORT_SIZE: usize = 1024;
pub const INPUT_SIZE: usize = 512;
const REPORT_HEADER_SIZE: usize = 4;
const IMAGE_HEADER_SIZE: usize = 8;
const IMAGE_CHUNK_SIZE: usize = REPORT_SIZE - IMAGE_HEADER_SIZE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedInputReport {
    Keys([bool; 8]),
    DialButtons([bool; 4]),
    DialRotate([i8; 4]),
    Touch(TouchEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    Tap {
        x: u16,
        y: u16,
        region: u8,
    },
    Press {
        x: u16,
        y: u16,
        region: u8,
    },
    Flick {
        start_x: u16,
        start_y: u16,
        end_x: u16,
        end_y: u16,
        region: u8,
    },
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub fn touch_region(x: u16) -> u8 {
    (((u32::from(x) * 4) / 800).min(3)) as u8
}

pub fn parse_input_report(report: &[u8]) -> Result<Option<ParsedInputReport>, String> {
    if report.len() < REPORT_HEADER_SIZE {
        return Err("Stream Deck input report is shorter than its header".into());
    }
    if report[0] != 0x01 {
        return Err(format!(
            "unexpected Stream Deck input report id 0x{:02x}",
            report[0]
        ));
    }
    let payload_len = usize::from(le_u16(&report[2..4]));
    if report.len() < REPORT_HEADER_SIZE + payload_len {
        return Err(format!(
            "Stream Deck input payload declares {payload_len} bytes but only {} are available",
            report.len().saturating_sub(REPORT_HEADER_SIZE)
        ));
    }
    let payload = &report[REPORT_HEADER_SIZE..REPORT_HEADER_SIZE + payload_len];
    match report[1] {
        0x00 => {
            if payload.len() < 8 {
                return Err("Stream Deck + key report requires 8 key states".into());
            }
            let mut keys = [false; 8];
            for (index, state) in keys.iter_mut().enumerate() {
                *state = payload[index] != 0;
            }
            Ok(Some(ParsedInputReport::Keys(keys)))
        }
        0x02 => parse_touch(payload).map(|touch| touch.map(ParsedInputReport::Touch)),
        0x03 => parse_dials(payload),
        _ => Ok(None),
    }
}

fn parse_touch(payload: &[u8]) -> Result<Option<TouchEvent>, String> {
    if payload.is_empty() {
        return Err("Stream Deck + touch report has no contents type".into());
    }
    match payload[0] {
        0x01 | 0x02 => {
            if payload.len() < 6 {
                return Err("Stream Deck + tap/press report is truncated".into());
            }
            let x = le_u16(&payload[2..4]);
            let y = le_u16(&payload[4..6]);
            let region = touch_region(x);
            if payload[0] == 0x01 {
                Ok(Some(TouchEvent::Tap { x, y, region }))
            } else {
                Ok(Some(TouchEvent::Press { x, y, region }))
            }
        }
        0x03 => {
            if payload.len() < 10 {
                return Err("Stream Deck + flick report is truncated".into());
            }
            let start_x = le_u16(&payload[2..4]);
            let start_y = le_u16(&payload[4..6]);
            let end_x = le_u16(&payload[6..8]);
            let end_y = le_u16(&payload[8..10]);
            Ok(Some(TouchEvent::Flick {
                start_x,
                start_y,
                end_x,
                end_y,
                region: touch_region(start_x),
            }))
        }
        _ => Ok(None),
    }
}

fn parse_dials(payload: &[u8]) -> Result<Option<ParsedInputReport>, String> {
    if payload.len() < 5 {
        return Err("Stream Deck + encoder report requires type plus four values".into());
    }
    match payload[0] {
        0x00 => {
            let mut buttons = [false; 4];
            for (index, state) in buttons.iter_mut().enumerate() {
                *state = payload[index + 1] != 0;
            }
            Ok(Some(ParsedInputReport::DialButtons(buttons)))
        }
        0x01 => {
            let mut ticks = [0i8; 4];
            for (index, tick) in ticks.iter_mut().enumerate() {
                *tick = payload[index + 1] as i8;
            }
            Ok(Some(ParsedInputReport::DialRotate(ticks)))
        }
        _ => Ok(None),
    }
}

fn image_reports(command: u8, selector: u8, jpeg: &[u8]) -> Result<Vec<[u8; REPORT_SIZE]>, String> {
    if jpeg.is_empty() {
        return Err("cannot upload an empty JPEG".into());
    }
    let chunks = jpeg.chunks(IMAGE_CHUNK_SIZE);
    let count = chunks.len();
    if count > usize::from(u16::MAX) + 1 {
        return Err("JPEG requires too many Stream Deck HID chunks".into());
    }
    let mut reports = Vec::with_capacity(count);
    for (index, chunk) in chunks.enumerate() {
        let mut report = [0u8; REPORT_SIZE];
        report[0] = 0x02;
        report[1] = command;
        report[2] = selector;
        report[3] = u8::from(index + 1 == count);
        report[4..6].copy_from_slice(&(chunk.len() as u16).to_le_bytes());
        report[6..8].copy_from_slice(&(index as u16).to_le_bytes());
        report[8..8 + chunk.len()].copy_from_slice(chunk);
        reports.push(report);
    }
    Ok(reports)
}

pub fn button_image_reports(index: u8, jpeg: &[u8]) -> Result<Vec<[u8; REPORT_SIZE]>, String> {
    if index >= 8 {
        return Err(format!(
            "Stream Deck + button index {index} is out of range"
        ));
    }
    image_reports(0x07, index, jpeg)
}

pub fn window_image_reports(jpeg: &[u8]) -> Result<Vec<[u8; REPORT_SIZE]>, String> {
    image_reports(0x0b, 0, jpeg)
}

pub fn brightness_report(percent: u8) -> Result<[u8; 32], String> {
    if percent > 100 {
        return Err("Stream Deck brightness must be between 0 and 100".into());
    }
    let mut report = [0u8; 32];
    report[0] = 0x03;
    report[1] = 0x08;
    report[2] = percent;
    Ok(report)
}

pub fn show_logo_report() -> [u8; 32] {
    let mut report = [0u8; 32];
    report[0] = 0x03;
    report[1] = 0x02;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_key_state_report() {
        let report = [0x01, 0x00, 0x08, 0x00, 1, 0, 1, 0, 0, 0, 0, 1];
        assert_eq!(
            parse_input_report(&report).unwrap(),
            Some(ParsedInputReport::Keys([
                true, false, true, false, false, false, false, true
            ]))
        );
    }

    #[test]
    fn parses_encoder_button_and_rotation_reports() {
        let buttons = [0x01, 0x03, 0x05, 0x00, 0x00, 1, 0, 1, 0];
        assert_eq!(
            parse_input_report(&buttons).unwrap(),
            Some(ParsedInputReport::DialButtons([true, false, true, false]))
        );

        let rotate = [0x01, 0x03, 0x05, 0x00, 0x01, 2, 0xff, 0, 0x80];
        assert_eq!(
            parse_input_report(&rotate).unwrap(),
            Some(ParsedInputReport::DialRotate([2, -1, 0, -128]))
        );
    }

    #[test]
    fn parses_touch_tap_press_and_flick_with_regions() {
        let tap = [
            0x01, 0x02, 0x0a, 0x00, 0x01, 0x00, 0x8a, 0x02, 0x32, 0x00, 0, 0, 0, 0,
        ];
        assert_eq!(
            parse_input_report(&tap).unwrap(),
            Some(ParsedInputReport::Touch(TouchEvent::Tap {
                x: 650,
                y: 50,
                region: 3
            }))
        );

        let press = [
            0x01, 0x02, 0x0a, 0x00, 0x02, 0x00, 0xc8, 0x00, 0x28, 0x00, 0, 0, 0, 0,
        ];
        assert_eq!(
            parse_input_report(&press).unwrap(),
            Some(ParsedInputReport::Touch(TouchEvent::Press {
                x: 200,
                y: 40,
                region: 1
            }))
        );

        let flick = [
            0x01, 0x02, 0x0e, 0x00, 0x03, 0x00, 0x64, 0x00, 0x20, 0x00, 0xbc, 0x02, 0x20, 0x00, 0,
            0, 0, 0,
        ];
        assert_eq!(
            parse_input_report(&flick).unwrap(),
            Some(ParsedInputReport::Touch(TouchEvent::Flick {
                start_x: 100,
                start_y: 32,
                end_x: 700,
                end_y: 32,
                region: 0,
            }))
        );
    }

    #[test]
    fn rejects_malformed_reports() {
        assert!(parse_input_report(&[0x02, 0x00, 0x00, 0x00]).is_err());
        assert!(parse_input_report(&[0x01, 0x00, 0x08, 0x00, 1, 0]).is_err());
        assert_eq!(parse_input_report(&[0x01, 0x7f, 0, 0]).unwrap(), None);
    }

    #[test]
    fn encodes_button_jpeg_chunks() {
        let jpeg = vec![0x5a; 1500];
        let reports = button_image_reports(3, &jpeg).unwrap();
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0][0..8], [0x02, 0x07, 3, 0, 0xf8, 0x03, 0, 0]);
        assert_eq!(reports[1][0], 0x02);
        assert_eq!(reports[1][1], 0x07);
        assert_eq!(reports[1][2], 3);
        assert_eq!(reports[1][3], 1);
        assert_eq!(u16::from_le_bytes([reports[1][4], reports[1][5]]), 484);
        assert_eq!(u16::from_le_bytes([reports[1][6], reports[1][7]]), 1);
        assert!(reports[1][8 + 484..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn encodes_window_jpeg_chunks() {
        let jpeg = vec![0x33; 1100];
        let reports = window_image_reports(&jpeg).unwrap();
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0][0..8], [0x02, 0x0b, 0, 0, 0xf8, 0x03, 0, 0]);
        assert_eq!(reports[1][3], 1);
        assert_eq!(u16::from_le_bytes([reports[1][4], reports[1][5]]), 84);
    }

    #[test]
    fn encodes_feature_reports() {
        let brightness = brightness_report(73).unwrap();
        assert_eq!(brightness[0..3], [0x03, 0x08, 73]);
        assert!(brightness[3..].iter().all(|byte| *byte == 0));
        assert!(brightness_report(101).is_err());

        let logo = show_logo_report();
        assert_eq!(logo[0..2], [0x03, 0x02]);
        assert!(logo[2..].iter().all(|byte| *byte == 0));
    }
}
