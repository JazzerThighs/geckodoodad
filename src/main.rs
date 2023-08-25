extern crate regex;

use std::fs;
use serde::Serialize;
use serde_json;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum GameVersion {
    NTSC(f64),
    KOR,
    PAL,
    Other,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum Category {
    Gameplay,
    Aesthetics,
    Features,
    
}

#[derive(Debug, Serialize)]
pub struct GeckoCode {
    header: String,
    version: Option<GameVersion>,
    authors: Option<Vec<String>>,
    description: Option<Vec<String>>,
    hex: Vec<String>,
    deprecated: bool,
    overwrite: Vec<String>,
    injection: Vec<String>,
    categories: Vec<String>,
}

impl GeckoCode {
    pub fn from_str(s: &str) -> Option<GeckoCode> {
        let re = Regex::new(r"(?x)
            ^\$
            (?P<header>.*?)\s*
            (?:\((?P<possible_version>[^\)]+)\))?\s*
            (?:\[(?P<authors>.*?)\])?\s*$
            (?P<description>(?:\n\*(?:.*?)$)*)
            (?P<hex>(?:$\n[\dA-Fa-fxyXY]{8}\s?[\dA-Fa-fxyXY]{8}\s*(?:#.*?)?$)+)
        ").unwrap();

        if let Some(caps) = re.captures(s) {
            let mut header = caps["header"].trim().to_string();

            let mut possible_version = caps.name("possible_version")
                .map(|m| m.as_str().trim().to_string());

            let authors = caps.name("authors")
                .map(|m| m.as_str().split(",").map(|author| author.trim().to_string()).collect::<Vec<String>>());

            let description = caps.name("description")
                .map(|m| m.as_str().lines().map(|line| line.trim().to_string()).collect::<Vec<String>>());

            let hex: Vec<String> = caps["hex"].lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, '#').collect();
                    Some(parts[0].trim().to_string())
                }).collect();

            // Check if possible_version is an actual version or part of the header
            let version = match possible_version.as_deref() {
                Some("1.0") | Some("1.00") => Some(GameVersion::NTSC(1.0)),
                Some("1.01") => Some(GameVersion::NTSC(1.01)),
                Some("1.02") => Some(GameVersion::NTSC(1.02)),
                Some("KOR") => Some(GameVersion::KOR),
                Some("PAL") => Some(GameVersion::PAL),
                Some(text) => {
                    // Append the text to the header as it's not a recognized version
                    header.push_str(&format!(" ({})", text));
                    None
                },
                None => None
            };

            Some(GeckoCode {
                header,
                version,
                authors,
                description,
                hex,
                deprecated: false,
                overwrite: Vec::new(),
                injection: Vec::new(),
                categories: Vec::new(),
            })
        } else {
            None
        }
    }
}

fn main() {
    let test_str = "YOUR_TEST_STRING_HERE";
    if let Some(gecko_code) = GeckoCode::from_str(test_str) {
        println!("{:?}", gecko_code);
    }
}
