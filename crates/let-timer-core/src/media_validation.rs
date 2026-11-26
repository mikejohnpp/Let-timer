use crate::db::models::MediaType;
use std::path::Path;

/// Metadata for a validated media file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMediaInfo {
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub max_size_bytes: u64,
}

/// Errors occurring during media file validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaValidationError {
    DisallowedExtension(String),
    MagicBytesMismatch(String),
    ProhibitedContent(String),
    Oversized { size: u64, max: u64 },
    InvalidUtf8,
    EmptyFile,
}

impl std::fmt::Display for MediaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaValidationError::DisallowedExtension(ext) => {
                write!(f, "disallowed media file extension: .{ext}")
            }
            MediaValidationError::MagicBytesMismatch(msg) => {
                write!(f, "magic bytes mismatch: {msg}")
            }
            MediaValidationError::ProhibitedContent(msg) => {
                write!(f, "prohibited media content: {msg}")
            }
            MediaValidationError::Oversized { size, max } => {
                write!(
                    f,
                    "file size {size} bytes exceeds maximum allowed {max} bytes"
                )
            }
            MediaValidationError::InvalidUtf8 => write!(f, "content is not valid UTF-8 text"),
            MediaValidationError::EmptyFile => write!(f, "media file is empty"),
        }
    }
}

impl std::error::Error for MediaValidationError {}

/// Get the maximum allowed upload size in bytes for a given media type,
/// taking into account the `LET_TIMER_MAX_UPLOAD_MB` environment override.
pub fn max_size_for_media_type(media_type: MediaType) -> u64 {
    let global_max_mb = std::env::var("LET_TIMER_MAX_UPLOAD_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(200);

    let type_default_mb = match media_type {
        MediaType::Image => 25,
        MediaType::Music => 200,
        MediaType::Poem => 1,
    };

    let effective_mb = type_default_mb.min(global_max_mb);
    effective_mb * 1024 * 1024
}

/// Lookup MIME type for an allowed media extension from a file path.
/// Returns None if the extension is not in the allow-list table.
pub fn mime_type_from_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        // Images
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        // Audio
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "aac" => Some("audio/aac"),
        "ogg" => Some("audio/ogg"),
        "opus" => Some("audio/opus"),
        "wav" => Some("audio/wav"),
        "flac" => Some("audio/flac"),
        // Poems
        "txt" => Some("text/plain; charset=utf-8"),
        "md" => Some("text/markdown; charset=utf-8"),
        _ => None,
    }
}

/// Validate a media file's extension, size, and header magic bytes.
pub fn validate_media(
    media_type: MediaType,
    filename: &str,
    header_bytes: &[u8],
    total_size: u64,
) -> Result<ValidatedMediaInfo, MediaValidationError> {
    if total_size == 0 {
        return Err(MediaValidationError::EmptyFile);
    }

    let max_size = max_size_for_media_type(media_type);
    if total_size > max_size {
        return Err(MediaValidationError::Oversized {
            size: total_size,
            max: max_size,
        });
    }

    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match media_type {
        MediaType::Image => validate_image(&ext, header_bytes, max_size),
        MediaType::Music => validate_audio(&ext, header_bytes, max_size),
        MediaType::Poem => validate_poem(&ext, header_bytes, max_size),
    }
}

fn validate_image(
    ext: &str,
    header: &[u8],
    max_size: u64,
) -> Result<ValidatedMediaInfo, MediaValidationError> {
    match ext {
        "png" => {
            // PNG signature: 89 50 4E 47 0D 0A 1A 0A
            const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            if header.len() < 8 || header[..8] != PNG_SIG {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid PNG file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "image/png",
                extension: "png",
                max_size_bytes: max_size,
            })
        }
        "jpg" | "jpeg" => {
            // JPEG starts with FF D8 FF
            if header.len() < 3 || header[0] != 0xFF || header[1] != 0xD8 || header[2] != 0xFF {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid JPEG file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "image/jpeg",
                extension: "jpg",
                max_size_bytes: max_size,
            })
        }
        "gif" => {
            // GIF starts with GIF87a or GIF89a
            if header.len() < 6 || (&header[..6] != b"GIF87a" && &header[..6] != b"GIF89a") {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid GIF file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "image/gif",
                extension: "gif",
                max_size_bytes: max_size,
            })
        }
        "webp" => {
            // RIFF....WEBP
            if header.len() < 12 || &header[..4] != b"RIFF" || &header[8..12] != b"WEBP" {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid WEBP file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "image/webp",
                extension: "webp",
                max_size_bytes: max_size,
            })
        }
        "avif" => {
            // ftyp at 4..8 and contains avif/avis/mif1
            if header.len() < 12 || &header[4..8] != b"ftyp" {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid AVIF container".into(),
                ));
            }
            let brand_area = &header[8..header.len().min(32)];
            let has_avif_brand = brand_area
                .windows(4)
                .any(|w| w == b"avif" || w == b"avis" || w == b"mif1");
            if !has_avif_brand {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid AVIF brand".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "image/avif",
                extension: "avif",
                max_size_bytes: max_size,
            })
        }
        _ => Err(MediaValidationError::DisallowedExtension(ext.to_string())),
    }
}

fn validate_audio(
    ext: &str,
    header: &[u8],
    max_size: u64,
) -> Result<ValidatedMediaInfo, MediaValidationError> {
    match ext {
        "mp3" => {
            // ID3v2 or MPEG sync word
            let is_id3 = header.len() >= 3 && &header[..3] == b"ID3";
            let is_mpeg_sync = header.len() >= 2 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0;
            if !is_id3 && !is_mpeg_sync {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid MP3 file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "audio/mpeg",
                extension: "mp3",
                max_size_bytes: max_size,
            })
        }
        "m4a" => {
            // ftyp at 4..8
            if header.len() < 12 || &header[4..8] != b"ftyp" {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid M4A container".into(),
                ));
            }
            let brand = &header[8..12];
            let is_m4a = brand == b"M4A " || brand == b"mp42" || brand == b"isom";
            if !is_m4a {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid M4A brand".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "audio/mp4",
                extension: "m4a",
                max_size_bytes: max_size,
            })
        }
        "aac" => {
            // ID3v2 or ADTS sync word (0xFF 0xF0..0xFF)
            let is_id3 = header.len() >= 3 && &header[..3] == b"ID3";
            let is_adts = header.len() >= 2 && header[0] == 0xFF && (header[1] & 0xF6) == 0xF0;
            if !is_id3 && !is_adts {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid AAC file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "audio/aac",
                extension: "aac",
                max_size_bytes: max_size,
            })
        }
        "ogg" | "opus" => {
            // OggS
            if header.len() < 4 || &header[..4] != b"OggS" {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid OGG container".into(),
                ));
            }
            let mime = if ext == "opus" {
                "audio/opus"
            } else {
                "audio/ogg"
            };
            Ok(ValidatedMediaInfo {
                mime_type: mime,
                extension: if ext == "opus" { "opus" } else { "ogg" },
                max_size_bytes: max_size,
            })
        }
        "wav" => {
            // RIFF....WAVE
            if header.len() < 12 || &header[..4] != b"RIFF" || &header[8..12] != b"WAVE" {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid WAV file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "audio/wav",
                extension: "wav",
                max_size_bytes: max_size,
            })
        }
        "flac" => {
            // fLaC
            if header.len() < 4 || &header[..4] != b"fLaC" {
                return Err(MediaValidationError::MagicBytesMismatch(
                    "not a valid FLAC file".into(),
                ));
            }
            Ok(ValidatedMediaInfo {
                mime_type: "audio/flac",
                extension: "flac",
                max_size_bytes: max_size,
            })
        }
        _ => Err(MediaValidationError::DisallowedExtension(ext.to_string())),
    }
}

fn validate_poem(
    ext: &str,
    header: &[u8],
    max_size: u64,
) -> Result<ValidatedMediaInfo, MediaValidationError> {
    if ext != "txt" && ext != "md" {
        return Err(MediaValidationError::DisallowedExtension(ext.to_string()));
    }

    let text = std::str::from_utf8(header).map_err(|_| MediaValidationError::InvalidUtf8)?;
    let text_lower = text.to_lowercase();

    // Check for dangerous active web / HTML tokens
    const FORBIDDEN_TOKENS: &[&str] = &[
        "<html",
        "<svg",
        "<script",
        "<?xml",
        "<!doctype",
        "<body",
        "<head",
        "<iframe",
        "<object",
        "<embed",
        "<link",
        "<meta",
        "javascript:",
    ];

    for forbidden in FORBIDDEN_TOKENS {
        if text_lower.contains(forbidden) {
            return Err(MediaValidationError::ProhibitedContent(format!(
                "contains forbidden token '{forbidden}'"
            )));
        }
    }

    let mime = if ext == "md" {
        "text/markdown; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    };

    Ok(ValidatedMediaInfo {
        mime_type: mime,
        extension: if ext == "md" { "md" } else { "txt" },
        max_size_bytes: max_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_image_formats() {
        // PNG
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0];
        let res = validate_media(MediaType::Image, "test.png", &png, 100).unwrap();
        assert_eq!(res.mime_type, "image/png");

        // JPEG
        let jpg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
        let res = validate_media(MediaType::Image, "test.jpg", &jpg, 100).unwrap();
        assert_eq!(res.mime_type, "image/jpeg");

        // GIF
        let gif = b"GIF89a...";
        let res = validate_media(MediaType::Image, "test.gif", gif, 100).unwrap();
        assert_eq!(res.mime_type, "image/gif");
    }

    #[test]
    fn test_rejects_html_and_svg_and_script() {
        let evil_html = b"<!DOCTYPE html><html><body><h1>test</h1></body></html>";
        // Attempt as poem
        assert!(validate_media(MediaType::Poem, "evil.txt", evil_html, 100).is_err());
        assert!(validate_media(MediaType::Poem, "evil.md", evil_html, 100).is_err());
        // Attempt as image disguised as png
        assert!(validate_media(MediaType::Image, "evil.png", evil_html, 100).is_err());

        // Attempt as svg extension
        assert!(validate_media(MediaType::Image, "evil.svg", b"<svg></svg>", 100).is_err());
        assert!(validate_media(MediaType::Poem, "evil.html", evil_html, 100).is_err());
    }

    #[test]
    fn test_valid_audio_formats() {
        let mp3_id3 = b"ID3\x03\x00\x00...";
        let res = validate_media(MediaType::Music, "track.mp3", mp3_id3, 500).unwrap();
        assert_eq!(res.mime_type, "audio/mpeg");

        let ogg = b"OggS\x00\x02...";
        let res = validate_media(MediaType::Music, "audio.ogg", ogg, 500).unwrap();
        assert_eq!(res.mime_type, "audio/ogg");

        let flac = b"fLaC\x00...";
        let res = validate_media(MediaType::Music, "song.flac", flac, 500).unwrap();
        assert_eq!(res.mime_type, "audio/flac");
    }

    #[test]
    fn test_oversized_media_rejected() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        // Image limit is 25MB (26_214_400 bytes)
        let res = validate_media(MediaType::Image, "test.png", &png, 30 * 1024 * 1024);
        assert!(matches!(res, Err(MediaValidationError::Oversized { .. })));
    }
}
