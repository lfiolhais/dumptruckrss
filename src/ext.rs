#[derive(Debug)]
pub enum AudioType {
    Mpeg,
    Aac,
    Mp4,
    Ogg,
    M4a,
}

impl AudioType {
    pub fn get_type_from_mime(mime_type: &str) -> Self {
        match mime_type {
            "audio/mpeg" => AudioType::Mpeg,
            "audio/aac" => AudioType::Aac,
            "audio/ogg" => AudioType::Ogg,
            "audio/mp4" => AudioType::Mp4,
            "audio/x-m4a" => AudioType::M4a,
            _ => panic!("Undetected Audio type: {}", mime_type),
        }
    }

    pub fn get_extension_from_type(ty: Self) -> &'static str {
        match ty {
            AudioType::Mpeg => "mp3",
            AudioType::Aac => "aac",
            AudioType::Ogg => "ogg",
            AudioType::Mp4 => "mp4",
            AudioType::M4a => "m4a",
        }
    }

    pub fn get_extension_from_mime(mime_type: &str) -> &'static str {
        let ty = Self::get_type_from_mime(mime_type);
        Self::get_extension_from_type(ty)
    }
}
