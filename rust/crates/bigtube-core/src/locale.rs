/// YouTube-compatible locale derived from the user's process locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubeLocale {
    pub language: String,
    pub region: Option<String>,
}

/// Read the user's preferred language and region from standard locale variables.
pub fn youtube_locale() -> YouTubeLocale {
    let locale = std::env::var("LANGUAGE")
        .ok()
        .and_then(|v| v.split(':').next().map(str::to_string))
        .filter(|v| !v.is_empty() && v != "C" && v != "POSIX")
        .or_else(|| {
            ["LC_ALL", "LC_MESSAGES", "LANG"]
                .into_iter()
                .filter_map(|key| std::env::var(key).ok())
                .find(|v| !v.is_empty() && !v.starts_with("C.") && v != "C" && v != "POSIX")
        });
    locale_from_name(locale.as_deref().unwrap_or("en"))
}

fn locale_from_name(locale: &str) -> YouTubeLocale {
    let name = locale.split(['.', '@']).next().unwrap_or(locale);
    if name.is_empty() || name == "C" || name == "POSIX" {
        return YouTubeLocale {
            language: "en".to_string(),
            region: None,
        };
    }
    let parts: Vec<_> = name.split(['-', '_']).filter(|p| !p.is_empty()).collect();
    let language = parts.first().copied().unwrap_or("en").to_ascii_lowercase();
    let region = parts
        .iter()
        .skip(1)
        .find(|part| {
            (part.len() == 2 && part.bytes().all(|b| b.is_ascii_alphabetic()))
                || (part.len() == 3 && part.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(|part| {
            if part.len() == 2 {
                part.to_ascii_uppercase()
            } else {
                part.to_string()
            }
        });
    let script = parts
        .iter()
        .skip(1)
        .find(|part| part.len() == 4 && part.bytes().all(|b| b.is_ascii_alphabetic()))
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                })
                .unwrap_or_default()
        });

    let language = match (language.as_str(), script.as_deref(), region.as_deref()) {
        ("he", _, _) => "iw".to_string(),
        ("tl", _, _) => "fil".to_string(),
        ("sr", Some("Latn"), _) => "sr-Latn".to_string(),
        ("pt", _, Some("PT")) => "pt-PT".to_string(),
        ("en", _, Some("GB")) => "en-GB".to_string(),
        ("en", _, Some("IN")) => "en-IN".to_string(),
        ("es", _, Some("419")) => "es-419".to_string(),
        ("zh", _, Some("TW")) => "zh-TW".to_string(),
        ("zh", _, Some("HK")) => "zh-HK".to_string(),
        ("zh", _, _) => "zh-CN".to_string(),
        (lang, _, _) => lang.to_string(),
    };

    let region = region.filter(|r| r.len() == 2);
    let language = if youtube_language_supported(&language) {
        language
    } else {
        "en".to_string()
    };
    YouTubeLocale { language, region }
}

fn youtube_language_supported(language: &str) -> bool {
    matches!(
        language,
        "af" | "am"
            | "as"
            | "az"
            | "be"
            | "bg"
            | "bn"
            | "bs"
            | "ca"
            | "cs"
            | "da"
            | "de"
            | "el"
            | "en"
            | "en-GB"
            | "en-IN"
            | "es"
            | "es-419"
            | "es-US"
            | "et"
            | "eu"
            | "fa"
            | "fi"
            | "fil"
            | "fr"
            | "fr-CA"
            | "gl"
            | "gu"
            | "he"
            | "hi"
            | "hr"
            | "hu"
            | "hy"
            | "id"
            | "is"
            | "it"
            | "iw"
            | "ja"
            | "ka"
            | "kk"
            | "km"
            | "kn"
            | "ko"
            | "ky"
            | "lo"
            | "lt"
            | "lv"
            | "mk"
            | "ml"
            | "mn"
            | "mr"
            | "ms"
            | "my"
            | "ne"
            | "nl"
            | "no"
            | "or"
            | "pa"
            | "pl"
            | "pt"
            | "pt-PT"
            | "ro"
            | "ru"
            | "si"
            | "sk"
            | "sl"
            | "sq"
            | "sr"
            | "sr-Latn"
            | "sv"
            | "sw"
            | "ta"
            | "te"
            | "th"
            | "tr"
            | "uk"
            | "ur"
            | "uz"
            | "vi"
            | "zh-CN"
            | "zh-HK"
            | "zh-TW"
            | "zu"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_linux_locales_for_youtube() {
        assert_eq!(
            locale_from_name("pt_BR.UTF-8"),
            YouTubeLocale {
                language: "pt".into(),
                region: Some("BR".into())
            }
        );
        assert_eq!(
            locale_from_name("pt_PT"),
            YouTubeLocale {
                language: "pt-PT".into(),
                region: Some("PT".into())
            }
        );
        assert_eq!(
            locale_from_name("sr_Latn_RS@latin"),
            YouTubeLocale {
                language: "sr-Latn".into(),
                region: Some("RS".into())
            }
        );
    }

    #[test]
    fn normalizes_youtube_legacy_language_codes() {
        assert_eq!(locale_from_name("he_IL").language, "iw");
        assert_eq!(locale_from_name("tl_PH").language, "fil");
        assert_eq!(locale_from_name("es_AR").language, "es");
        assert_eq!(locale_from_name("zh_TW").language, "zh-TW");
    }

    #[test]
    fn unknown_and_c_locales_fall_back_to_english() {
        assert_eq!(locale_from_name("C.UTF-8").language, "en");
        assert_eq!(locale_from_name("").language, "en");
        assert_eq!(locale_from_name("xx_YY").language, "en");
    }
}
