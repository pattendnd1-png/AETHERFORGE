use crate::MtfError;

pub(crate) fn decompress_payload(payload: &[u8], expected: usize) -> Result<Vec<u8>, MtfError> {
    let mut out = Vec::with_capacity(expected);
    let mut at = 0usize;

    while out.len() < expected {
        let flags = take_u8(payload, &mut at)?;
        for bit in 0..8 {
            if out.len() == expected {
                break;
            }

            if flags & (1u8 << bit) != 0 {
                let byte = take_u8(payload, &mut at)?;
                out.push(byte);
                continue;
            }

            let word = take_u16(payload, &mut at)?;
            let count = usize::from(word >> 10) + 3;
            let offset = word & 0x03ff;
            if offset == 0 || usize::from(offset) > out.len() {
                return Err(MtfError::InvalidBackReference {
                    offset,
                    produced: out.len(),
                });
            }

            let attempted = out
                .len()
                .checked_add(count)
                .ok_or(MtfError::ArithmeticOverflow)?;
            if attempted > expected {
                return Err(MtfError::OutputOverflow {
                    produced: out.len(),
                    attempted,
                    expected,
                });
            }

            for _ in 0..count {
                let source = out.len().checked_sub(usize::from(offset)).ok_or(
                    MtfError::InvalidBackReference {
                        offset,
                        produced: out.len(),
                    },
                )?;
                let byte = out[source];
                out.push(byte);
            }
        }
    }

    Ok(out)
}

fn take_u8(bytes: &[u8], at: &mut usize) -> Result<u8, MtfError> {
    let value = bytes
        .get(*at)
        .copied()
        .ok_or(MtfError::CompressedPayloadEof { offset: *at })?;
    *at = (*at).checked_add(1).ok_or(MtfError::ArithmeticOverflow)?;
    Ok(value)
}

fn take_u16(bytes: &[u8], at: &mut usize) -> Result<u16, MtfError> {
    let end = at.checked_add(2).ok_or(MtfError::ArithmeticOverflow)?;
    let raw = bytes
        .get(*at..end)
        .ok_or(MtfError::CompressedPayloadEof { offset: *at })?;
    *at = end;
    Ok(u16::from_le_bytes(raw.try_into().unwrap()))
}
