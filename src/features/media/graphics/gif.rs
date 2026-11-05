pub(super) fn gif_frame_delays(bytes: &[u8]) -> Result<Vec<i64>, String> {
    if bytes.len() < 13 || !bytes.starts_with(b"GIF87a") && !bytes.starts_with(b"GIF89a") {
        return Err("arquivo GIF inválido".to_owned());
    }
    let packed = bytes[10];
    let mut offset = 13usize;
    if packed & 0x80 != 0 {
        offset = offset
            .checked_add(3 * (1usize << (usize::from(packed & 0x07) + 1)))
            .ok_or_else(|| "arquivo GIF inválido".to_owned())?;
    }
    let mut next_delay = 100i64;
    let mut delays = Vec::new();
    while offset < bytes.len() {
        match bytes[offset] {
            0x3b => return Ok(delays),
            0x21 => {
                let label = *bytes
                    .get(offset + 1)
                    .ok_or_else(|| "extensão GIF incompleta".to_owned())?;
                offset += 2;
                if label == 0xf9 {
                    let size = *bytes
                        .get(offset)
                        .ok_or_else(|| "controle de quadro GIF incompleto".to_owned())?
                        as usize;
                    if size != 4 || offset + size + 2 > bytes.len() {
                        return Err("controle de quadro GIF inválido".to_owned());
                    }
                    let centiseconds = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                    next_delay = i64::from(centiseconds.max(1)) * 10;
                    offset += size + 2;
                } else {
                    skip_gif_sub_blocks(bytes, &mut offset)?;
                }
            }
            0x2c => {
                if offset + 10 > bytes.len() {
                    return Err("descritor de quadro GIF incompleto".to_owned());
                }
                let image_packed = bytes[offset + 9];
                offset += 10;
                if image_packed & 0x80 != 0 {
                    offset = offset
                        .checked_add(3 * (1usize << (usize::from(image_packed & 0x07) + 1)))
                        .ok_or_else(|| "arquivo GIF inválido".to_owned())?;
                }
                if offset >= bytes.len() {
                    return Err("dados de quadro GIF incompletos".to_owned());
                }
                offset += 1;
                skip_gif_sub_blocks(bytes, &mut offset)?;
                delays.push(next_delay);
                next_delay = 100;
            }
            _ => return Err("bloco GIF inválido".to_owned()),
        }
    }
    Err("arquivo GIF sem terminador".to_owned())
}

pub(super) fn skip_gif_sub_blocks(bytes: &[u8], offset: &mut usize) -> Result<(), String> {
    loop {
        let size = *bytes
            .get(*offset)
            .ok_or_else(|| "bloco GIF incompleto".to_owned())? as usize;
        *offset += 1;
        if size == 0 {
            return Ok(());
        }
        *offset = offset
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| "bloco GIF incompleto".to_owned())?;
    }
}
