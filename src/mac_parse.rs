use dotenvy_macro::dotenv;

// 1. Hilfsfunktion für die Kompilierzeit (parst ein einzelnes Hex-Zeichen)
const fn parse_hex_char(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("Ungueltiges Zeichen in der MAC-Adresse"),
    }
}

// 2. Hauptfunktion für die Kompilierzeit (parst den ganzen String)
const fn parse_mac_const(mac_str: &str) -> [u8; 6] {
    let bytes = mac_str.as_bytes();

    // Eine MAC-Adresse im Format XX:XX:XX:XX:XX:XX hat exakt 17 Zeichen
    if bytes.len() != 17 {
        panic!("MAC-Adresse muss das Format XX:XX:XX:XX:XX:XX haben");
    }

    [
        (parse_hex_char(bytes[0]) << 4) | parse_hex_char(bytes[1]),
        (parse_hex_char(bytes[3]) << 4) | parse_hex_char(bytes[4]),
        (parse_hex_char(bytes[6]) << 4) | parse_hex_char(bytes[7]),
        (parse_hex_char(bytes[9]) << 4) | parse_hex_char(bytes[10]),
        (parse_hex_char(bytes[12]) << 4) | parse_hex_char(bytes[13]),
        (parse_hex_char(bytes[15]) << 4) | parse_hex_char(bytes[16]),
    ]
}

// Public machen für andere Dateien - main:
pub const PC_MAC: [u8; 6] = parse_mac_const(dotenv!("MAC"));