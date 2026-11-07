//! The SA-MP and open.mp server query protocol (the `i` information packet).

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use serde::Serialize;

/// Largest answer accepted; a real one is under 600 bytes.
const MAX_PACKET_BYTES: usize = 2048;

/// What a SA-MP server says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryInfo {
    /// The server asks for a password to join.
    pub password: bool,
    /// Players connected now.
    pub players: u16,
    /// Player slots.
    pub max_players: u16,
    /// Server name, without `{RRGGBB}` color tags.
    pub hostname: String,
    /// Game mode name.
    pub gamemode: String,
    /// Language the server declares.
    pub language: String,
}

/// Windows-1252 characters for bytes `0x80..=0x9F`, which Latin-1 leaves as controls.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}', '\u{017D}', '\u{FFFD}',
    '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
];

/// Decodes text that may be UTF-8 or Windows-1252, stopping at a NUL.
pub fn decode_samp_bytes(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let slice = &bytes[..end];
    let text = match std::str::from_utf8(slice) {
        Ok(utf8) => utf8.to_owned(),
        Err(_) => slice
            .iter()
            .map(|&b| match b {
                0x80..=0x9F => CP1252_HIGH[usize::from(b - 0x80)],
                _ => char::from(b),
            })
            .collect(),
    };
    strip_color_codes(&text)
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Removes inline `{RRGGBB}` color tags.
pub fn strip_color_codes(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'{'
            && index + 7 < bytes.len()
            && bytes[index + 7] == b'}'
            && bytes[index + 1..index + 7]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            index += 8;
            continue;
        }
        // `index` is always on a character boundary: tags are ASCII.
        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// The 11-byte `i` request for a server.
pub fn build_info_packet(ip: Ipv4Addr, port: u16) -> [u8; 11] {
    let o = ip.octets();
    let p = port.to_le_bytes();
    [
        b'S', b'A', b'M', b'P', o[0], o[1], o[2], o[3], p[0], p[1], b'i',
    ]
}

/// Reads the answer to an `i` request.
pub fn parse_info_packet(
    packet: &[u8],
    expected_ip: Ipv4Addr,
    expected_port: u16,
) -> Option<QueryInfo> {
    let header = build_info_packet(expected_ip, expected_port);
    if packet.len() < 11 + 1 + 2 + 2 + 12 || packet[..11] != header {
        return None;
    }
    let mut cursor = 11;
    let password = packet[cursor] != 0;
    cursor += 1;
    let players = u16::from_le_bytes([packet[cursor], packet[cursor + 1]]);
    cursor += 2;
    let max_players = u16::from_le_bytes([packet[cursor], packet[cursor + 1]]);
    cursor += 2;
    let hostname = read_len_string(packet, &mut cursor)?;
    let gamemode = read_len_string(packet, &mut cursor)?;
    let language = read_len_string(packet, &mut cursor)?;
    Some(QueryInfo {
        password,
        players,
        max_players,
        hostname,
        gamemode,
        language,
    })
}

fn read_len_string(packet: &[u8], cursor: &mut usize) -> Option<String> {
    let length_end = cursor.checked_add(4)?;
    let length_bytes: [u8; 4] = packet.get(*cursor..length_end)?.try_into().ok()?;
    let len = u32::from_le_bytes(length_bytes) as usize;
    let end = length_end.checked_add(len)?;
    let text = decode_samp_bytes(packet.get(length_end..end)?);
    *cursor = end;
    Some(text)
}

/// Sends one `i` request and reads the answer.
///
/// Returns:
///     `None` if it did not answer in time, the answer came from elsewhere,
///     or it was malformed.
pub fn query_udp(addr: SocketAddrV4, timeout: Duration) -> Option<QueryInfo> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.set_read_timeout(Some(timeout)).ok()?;
    socket.set_write_timeout(Some(timeout)).ok()?;
    socket
        .send_to(&build_info_packet(*addr.ip(), addr.port()), addr)
        .ok()?;
    let mut buffer = [0_u8; MAX_PACKET_BYTES];
    let (size, from) = socket.recv_from(&mut buffer).ok()?;
    if from != SocketAddr::V4(addr) {
        return None;
    }
    parse_info_packet(&buffer[..size], *addr.ip(), addr.port())
}

/// An answer as a server would send it, for tests here and in [`crate::query`].
#[cfg(test)]
pub(crate) fn sample_answer(ip: Ipv4Addr, port: u16, fields: [&str; 3]) -> Vec<u8> {
    let mut packet = Vec::from(build_info_packet(ip, port));
    packet.push(0);
    packet.extend_from_slice(&42_u16.to_le_bytes());
    packet.extend_from_slice(&200_u16.to_le_bytes());
    for field in fields {
        packet.extend_from_slice(&(field.len() as u32).to_le_bytes());
        packet.extend_from_slice(field.as_bytes());
    }
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_tags_and_control_characters_are_removed() {
        assert_eq!(
            strip_color_codes("{FF00AA}Los Santos {ffffff}Roleplay {ZZZZZZ}"),
            "Los Santos Roleplay {ZZZZZZ}"
        );
        // 0x96 is an en dash and 0xF1 is ñ in Windows-1252.
        let raw = b"{00FF00}Espa\xF1ol \x96 RP\0trailing garbage";
        assert_eq!(decode_samp_bytes(raw), "Español \u{2013} RP");
        assert_eq!(decode_samp_bytes(b"Bad\x07\x1b[31mName\n"), "Bad[31mName");
    }

    #[test]
    fn answers_are_read_with_every_length_checked() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let packet = sample_answer(ip, 7777, ["{FF6600}SA-MP Server", "Roleplay v2", "Español"]);
        let parsed = parse_info_packet(&packet, ip, 7777).expect("valid packet");
        assert_eq!(parsed.hostname, "SA-MP Server");
        assert_eq!((parsed.players, parsed.max_players), (42, 200));
        assert_eq!(parsed.language, "Español");

        // Another port, a cut answer or a huge length are refused, never a panic.
        assert_eq!(parse_info_packet(&packet, ip, 7778), None);
        for cut in [0, 10, 11, 20, packet.len() - 1] {
            assert_eq!(parse_info_packet(&packet[..cut], ip, 7777), None, "{cut}");
        }
        let mut huge = Vec::from(build_info_packet(ip, 7777));
        huge.extend_from_slice(&[0, 1, 0, 10, 0]);
        huge.extend_from_slice(&u32::MAX.to_le_bytes());
        huge.extend_from_slice(&[0; 12]);
        assert_eq!(parse_info_packet(&huge, ip, 7777), None);
    }
}

#[cfg(test)]
mod random_input {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_reply_gives_an_answer_or_nothing(packet in proptest::collection::vec(any::<u8>(), 0..400)) {
            let _ = parse_info_packet(&packet, Ipv4Addr::LOCALHOST, 7777);
        }

        // A reply that passes the header reaches the length fields, which a
        // server can set to anything.
        #[test]
        fn lengths_past_the_end_are_safe(body in proptest::collection::vec(any::<u8>(), 0..200)) {
            let mut packet = build_info_packet(Ipv4Addr::LOCALHOST, 7777).to_vec();
            packet.extend(body);
            let _ = parse_info_packet(&packet, Ipv4Addr::LOCALHOST, 7777);
        }

        #[test]
        fn any_text_is_safe_with_color_codes(text in "\\PC{0,200}", bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
            let _ = strip_color_codes(&text);
            let _ = strip_color_codes(&format!("{{{text}}}"));
            let _ = decode_samp_bytes(&bytes);
        }
    }
}
