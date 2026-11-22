//! QR codes: encoding text, Wi-Fi join codes, and exporting as SVG or PNG.

use qrcode::types::{Color, QrError};
use qrcode::{EcLevel, QrCode, Version};

use crate::{CoreError, CoreResult};

/// The blank border scanners need, in modules; the QR specification asks for four.
pub const DEFAULT_QUIET_ZONE: usize = 4;
/// Larger PNGs only take longer to write and save.
pub const MAX_PNG_SIZE: u32 = 4096;

pub type Rgb = [u8; 3];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ErrorCorrection {
    Low,
    #[default]
    Medium,
    Quartile,
    High,
}

impl ErrorCorrection {
    pub const ALL: [ErrorCorrection; 4] = [
        ErrorCorrection::Low,
        ErrorCorrection::Medium,
        ErrorCorrection::Quartile,
        ErrorCorrection::High,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            ErrorCorrection::Low => "Low (7%)",
            ErrorCorrection::Medium => "Medium (15%)",
            ErrorCorrection::Quartile => "Quartile (25%)",
            ErrorCorrection::High => "High (30%)",
        }
    }

    const fn level(self) -> EcLevel {
        match self {
            ErrorCorrection::Low => EcLevel::L,
            ErrorCorrection::Medium => EcLevel::M,
            ErrorCorrection::Quartile => EcLevel::Q,
            ErrorCorrection::High => EcLevel::H,
        }
    }

    /// The most bytes a version 40 code holds at this level.
    const fn capacity_bytes(self) -> usize {
        match self {
            ErrorCorrection::Low => 2953,
            ErrorCorrection::Medium => 2331,
            ErrorCorrection::Quartile => 1663,
            ErrorCorrection::High => 1273,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WifiSecurity {
    #[default]
    Wpa,
    Wep,
    Open,
}

impl WifiSecurity {
    pub const ALL: [WifiSecurity; 3] = [WifiSecurity::Wpa, WifiSecurity::Wep, WifiSecurity::Open];

    pub const fn label(self) -> &'static str {
        match self {
            WifiSecurity::Wpa => "WPA / WPA2 / WPA3",
            WifiSecurity::Wep => "WEP",
            WifiSecurity::Open => "None",
        }
    }

    const fn code(self) -> &'static str {
        match self {
            WifiSecurity::Wpa => "WPA",
            WifiSecurity::Wep => "WEP",
            WifiSecurity::Open => "nopass",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub password: String,
    pub security: WifiSecurity,
    pub is_hidden: bool,
}

/// The modules of an encoded QR code, without its quiet zone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QrMatrix {
    pub width: usize,
    /// 1 to 40.
    pub version: i16,
    modules: Vec<bool>,
}

impl QrMatrix {
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        x < self.width
            && self
                .modules
                .get(y * self.width + x)
                .copied()
                .unwrap_or(false)
    }
}

/// How an exported code looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QrStyle {
    pub quiet_zone: usize,
    pub dark: Rgb,
    pub light: Rgb,
}

impl Default for QrStyle {
    fn default() -> Self {
        Self {
            quiet_zone: DEFAULT_QUIET_ZONE,
            dark: [0, 0, 0],
            light: [255, 255, 255],
        }
    }
}

pub fn encode_qr(text: &str, correction: ErrorCorrection) -> CoreResult<QrMatrix> {
    if text.is_empty() {
        return Err(CoreError::InvalidInput(
            "Enter something to encode.".to_owned(),
        ));
    }
    let code = QrCode::with_error_correction_level(text, correction.level())
        .map_err(|error| describe_qr_error(error, text, correction))?;
    let version = match code.version() {
        Version::Normal(number) | Version::Micro(number) => number,
    };
    Ok(QrMatrix {
        width: code.width(),
        version,
        modules: code
            .to_colors()
            .into_iter()
            .map(|color| color == Color::Dark)
            .collect(),
    })
}

/// The text phones read to join a Wi-Fi network, in the de facto `WIFI:` format.
pub fn build_wifi_payload(network: &WifiNetwork) -> String {
    let mut payload = format!(
        "WIFI:T:{};S:{};",
        network.security.code(),
        escape_wifi_field(&network.ssid)
    );
    if network.security != WifiSecurity::Open {
        payload.push_str("P:");
        payload.push_str(&escape_wifi_field(&network.password));
        payload.push(';');
    }
    if network.is_hidden {
        payload.push_str("H:true;");
    }
    payload.push(';');
    payload
}

/// A scalable image, one unit per module, drawn as a single path.
pub fn render_svg(matrix: &QrMatrix, style: QrStyle, pixel_size: u32) -> String {
    let size = matrix.width + style.quiet_zone * 2;
    let mut path = String::new();
    for y in 0..matrix.width {
        for x in 0..matrix.width {
            if matrix.is_dark(x, y) {
                push_module(&mut path, x + style.quiet_zone, y + style.quiet_zone);
            }
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pixel_size}\" height=\"{pixel_size}\" \
viewBox=\"0 0 {size} {size}\" shape-rendering=\"crispEdges\">\
<rect width=\"{size}\" height=\"{size}\" fill=\"{light}\"/>\
<path fill=\"{dark}\" d=\"{path}\"/></svg>\n",
        light = format_hex_color(style.light),
        dark = format_hex_color(style.dark),
    )
}

/// A PNG close to `target_size` pixels square. Each module gets a whole number of pixels, so
/// the result can be a little smaller than asked for, but never blurry.
pub fn render_png(matrix: &QrMatrix, style: QrStyle, target_size: u32) -> CoreResult<Vec<u8>> {
    let modules = u32::try_from(matrix.width + style.quiet_zone * 2)
        .map_err(|_| CoreError::InvalidInput("The QR code is too large.".to_owned()))?;
    let scale = (target_size.min(MAX_PNG_SIZE) / modules).max(1);
    let size = modules * scale;
    let quiet_zone = style.quiet_zone;
    let mut pixels = Vec::with_capacity((size * size * 3) as usize);
    for pixel_y in 0..size {
        for pixel_x in 0..size {
            let module_x = (pixel_x / scale) as usize;
            let module_y = (pixel_y / scale) as usize;
            let is_dark = module_x >= quiet_zone
                && module_y >= quiet_zone
                && matrix.is_dark(module_x - quiet_zone, module_y - quiet_zone);
            pixels.extend_from_slice(if is_dark { &style.dark } else { &style.light });
        }
    }
    encode_png(&pixels, size)
}

/// One square module as an SVG path segment.
fn push_module(path: &mut String, left: usize, top: usize) {
    path.push('M');
    path.push_str(&left.to_string());
    path.push(' ');
    path.push_str(&top.to_string());
    path.push_str("h1v1h-1z");
}

pub fn format_hex_color(color: Rgb) -> String {
    let [red, green, blue] = color;
    format!("#{red:02x}{green:02x}{blue:02x}")
}

fn encode_png(pixels: &[u8], size: u32) -> CoreResult<Vec<u8>> {
    let to_error = |error: png::EncodingError| {
        CoreError::InvalidInput(format!("Couldn't write the PNG: {error}."))
    };
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, size, size);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(to_error)?;
    writer.write_image_data(pixels).map_err(to_error)?;
    writer.finish().map_err(to_error)?;
    Ok(bytes)
}

fn describe_qr_error(error: QrError, text: &str, correction: ErrorCorrection) -> CoreError {
    let message = match error {
        QrError::DataTooLong => format!(
            "Too much to fit: {} bytes, but a QR code holds at most {} at {} error correction.",
            text.len(),
            correction.capacity_bytes(),
            correction.label()
        ),
        other => format!("Couldn't make a QR code: {other}."),
    };
    CoreError::InvalidInput(message)
}

fn escape_wifi_field(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | ';' | ',' | ':' | '"') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_text_into_a_square_matrix() {
        let matrix = encode_qr("https://example.com", ErrorCorrection::Medium).unwrap();
        assert_eq!(matrix.version, 2);
        assert_eq!(matrix.width, 25);
        // Finder patterns: dark corners, with a light ring inside.
        assert!(matrix.is_dark(0, 0));
        assert!(!matrix.is_dark(1, 1));
        assert!(matrix.is_dark(24, 0));
        assert!(!matrix.is_dark(25, 0));
    }

    #[test]
    fn higher_correction_needs_a_bigger_code() {
        let text = "The quick brown fox jumps over the lazy dog";
        let low = encode_qr(text, ErrorCorrection::Low).unwrap();
        let high = encode_qr(text, ErrorCorrection::High).unwrap();
        assert!(high.width > low.width);
    }

    #[test]
    fn explains_empty_and_oversized_input() {
        assert_eq!(
            encode_qr("", ErrorCorrection::Low).unwrap_err().to_string(),
            "Enter something to encode."
        );
        let long = "x".repeat(3000);
        assert_eq!(
            encode_qr(&long, ErrorCorrection::High)
                .unwrap_err()
                .to_string(),
            "Too much to fit: 3000 bytes, but a QR code holds at most 1273 at High (30%) error correction."
        );
    }

    #[test]
    fn wifi_payloads_escape_special_characters() {
        let network = WifiNetwork {
            ssid: "Cafe;Guest".to_owned(),
            password: "p:a\\ss".to_owned(),
            security: WifiSecurity::Wpa,
            is_hidden: true,
        };
        assert_eq!(
            build_wifi_payload(&network),
            "WIFI:T:WPA;S:Cafe\\;Guest;P:p\\:a\\\\ss;H:true;;"
        );
        let open = WifiNetwork {
            ssid: "Lobby".to_owned(),
            password: "ignored".to_owned(),
            security: WifiSecurity::Open,
            is_hidden: false,
        };
        assert_eq!(build_wifi_payload(&open), "WIFI:T:nopass;S:Lobby;;");
    }

    #[test]
    fn svg_draws_each_dark_module_inside_the_quiet_zone() {
        let matrix = encode_qr("hi", ErrorCorrection::Low).unwrap();
        let svg = render_svg(&matrix, QrStyle::default(), 300);
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\""));
        assert!(svg.contains("viewBox=\"0 0 29 29\""));
        assert!(svg.contains("fill=\"#ffffff\""));
        assert!(svg.contains("M4 4h1v1h-1z"));
        let dark_modules = (0..matrix.width)
            .flat_map(|y| (0..matrix.width).map(move |x| (x, y)))
            .filter(|&(x, y)| matrix.is_dark(x, y))
            .count();
        assert_eq!(svg.matches("h1v1h-1z").count(), dark_modules);
    }

    #[test]
    fn png_uses_whole_pixels_per_module() {
        let matrix = encode_qr("hi", ErrorCorrection::Low).unwrap();
        let bytes = render_png(&matrix, QrStyle::default(), 300).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
        let reader = decoder.read_info().unwrap();
        // 29 modules at 10 pixels each, the most that fit in 300.
        assert_eq!(reader.info().width, 290);
        assert_eq!(reader.info().height, 290);
    }

    #[test]
    fn formats_colours_as_hex() {
        assert_eq!(format_hex_color([255, 16, 0]), "#ff1000");
    }
}
