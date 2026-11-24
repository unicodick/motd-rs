use std::io;

use serde::Serialize;

use tokio::io::{AsyncRead, AsyncReadExt};

pub(crate) const MIN_PROTOCOL: i32 = 4; // 1.7.2
pub(crate) const DEFAULT_STATUS_PROTOCOL: i32 = 777; // 26.3
pub(crate) const MAX_PACKET: usize = 1024;
const MAX_JSON_SIZE: usize = 32_767;

const KEY_PROTOCOL: i32 = 759; // 1.19
const OPTIONAL_UUID_PROTOCOL: i32 = 760; // 1.19.1
const NO_KEY_PROTOCOL: i32 = 761; // 1.19.3
const REQUIRED_UUID_PROTOCOL: i32 = 764; // 1.20.2
const TRANSFER_PROTOCOL: i32 = 766; // 1.20.5

pub(crate) struct Handshake {
    pub(crate) protocol: i32,
    pub(crate) intent: Intent,
}

pub(crate) enum Intent {
    Status,
    Login,
}

pub(crate) fn handshake(packet: &[u8]) -> Option<Handshake> {
    let [0, payload @ ..] = packet else {
        return None;
    };
    let (protocol, rest) = take_varint(payload)?;
    let (host, rest) = take_string(rest, 255)?;
    if host.is_empty() {
        return None;
    }

    let (intent, []) = take_varint(rest.get(2..)?)? else {
        return None;
    };
    let intent = match intent {
        1 => Intent::Status,
        2 if protocol >= MIN_PROTOCOL => Intent::Login,
        3 if protocol >= TRANSFER_PROTOCOL => Intent::Login,
        _ => return None,
    };
    Some(Handshake { protocol, intent })
}

pub(crate) fn login_start(packet: &[u8], protocol: i32) -> Option<&str> {
    let [0, payload @ ..] = packet else {
        return None;
    };
    let (player, mut rest) = take_string(payload, 16)?;
    if player.is_empty() {
        return None;
    }

    if (KEY_PROTOCOL..NO_KEY_PROTOCOL).contains(&protocol) {
        rest = match rest {
            [0, rest @ ..] => rest,
            [1, rest @ ..] => {
                let (_, rest) = take_bytes(rest.get(8..)?, 294)?; // key expiry and key
                let (_, rest) = take_bytes(rest, 512)?; // signature
                rest
            }
            _ => return None,
        };
    }
    let valid = match protocol {
        REQUIRED_UUID_PROTOCOL.. => rest.len() == 16,
        OPTIONAL_UUID_PROTOCOL.. => rest == [0] || (rest.len() == 17 && rest[0] == 1),
        _ => rest.is_empty(),
    };
    valid.then_some(player)
}

pub(crate) async fn read_packet<'a>(
    reader: &mut (impl AsyncRead + Unpin),
    buf: &'a mut [u8; MAX_PACKET],
) -> io::Result<&'a [u8]> {
    let mut size = 0;
    // frame lengths fit in two varint bytes
    for shift in [0, 7] {
        let byte = reader.read_u8().await?;
        size |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            if size == 0 || size > buf.len() {
                break;
            }
            reader.read_exact(&mut buf[..size]).await?;
            return Ok(&buf[..size]);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid packet length",
    ))
}

pub(crate) fn json_packet(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let json = serde_json::to_vec(value)?;
    if json.len() > MAX_JSON_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "response json too large",
        ));
    }

    let mut string_prefix = [0; 5];
    let string_len = encode_varint(json.len(), &mut string_prefix);
    let payload_len = 1 + string_len.len() + json.len();
    let mut frame_prefix = [0; 5];
    let frame_len = encode_varint(payload_len, &mut frame_prefix);

    let mut packet = Vec::with_capacity(frame_len.len() + payload_len);
    packet.extend_from_slice(frame_len);
    packet.push(0); // id 0 for status and login disconnect
    packet.extend_from_slice(string_len);
    packet.extend_from_slice(&json);
    Ok(packet)
}

fn encode_varint(mut value: usize, output: &mut [u8; 5]) -> &[u8] {
    let mut index = 0;
    while value >= 0x80 {
        output[index] = (value as u8 & 0x7f) | 0x80;
        value >>= 7;
        index += 1;
    }
    output[index] = value as u8;
    &output[..=index]
}

fn take_varint(data: &[u8]) -> Option<(i32, &[u8])> {
    let mut value = 0u32;
    for (index, &byte) in data.iter().take(5).enumerate() {
        if index == 4 && byte > 0x0f {
            break;
        }
        value |= u32::from(byte & 0x7f) << (7 * index);
        if byte & 0x80 == 0 {
            return Some((value as i32, &data[index + 1..]));
        }
    }
    None
}

fn take_string(data: &[u8], limit: usize) -> Option<(&str, &[u8])> {
    let (bytes, rest) = take_bytes(data, limit * 3)?;
    let value = std::str::from_utf8(bytes).ok()?;
    if value.encode_utf16().count() > limit {
        return None;
    }
    Some((value, rest))
}

fn take_bytes(data: &[u8], limit: usize) -> Option<(&[u8], &[u8])> {
    let (size, rest) = take_varint(data)?;
    let size = usize::try_from(size).ok()?;
    if size > limit {
        return None;
    }
    rest.split_at_checked(size)
}
