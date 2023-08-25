extern crate regex;

use regex::Regex;
use serde::Serialize;
use serde_json;
// use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

#[derive(Debug, PartialEq, Clone, Serialize)]
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
        let re = Regex::new(
            r"(?x)
            ^\$
            (?P<header>.*?)\s*
            (?:\((?P<possible_version>[^\)]+)\))?\s*
            (?:\[(?P<authors>.*?)\])?\s*$
            (?P<description>(?:\n\*(?:.*?)$)*)
            (?P<hex>(?:$\n[\dA-Fa-fxyXY]{8}\s?[\dA-Fa-fxyXY]{8}\s*(?:#.*?)?$)+)
        ",
        )
        .unwrap();

        if let Some(caps) = re.captures(s) {
            let mut header = caps["header"].trim().to_string();

            let possible_version = caps
                .name("possible_version")
                .map(|m| m.as_str().trim().to_string());

            let authors = caps.name("authors").map(|m| {
                m.as_str()
                    .split(",")
                    .map(|author| author.trim().to_string())
                    .collect::<Vec<String>>()
            });

            let description = caps.name("description").map(|m| {
                m.as_str()
                    .lines()
                    .map(|line| line.trim().to_string())
                    .collect::<Vec<String>>()
            });

            let hex: Vec<String> = caps["hex"]
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, '#').collect();
                    Some(parts[0].trim().to_string())
                })
                .collect();

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
                }
                None => None,
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

fn extract_gecko_codes(input: &str) -> Vec<GeckoCode> {
    input                                                   // Split by the dollar sign indicating a new Gecko Code. Keep in mind that this might not handle the very first Gecko Code correctly, which we'll handle separately.
        .split("\n$") 
        .filter_map(|block| {
            
            let adjusted_block = format!("${}", block); // We prepend a dollar sign because we split by "\n$" and lost the leading "$".
            GeckoCode::from_str(&adjusted_block)
        })
        .collect()
}

fn main() {
    let file_path = Path::new("../geckoCodeWikiPage.md"); // Read the markdown file
    let file_content = fs::read_to_string(&file_path).expect("Unable to read file");

    let mut gecko_codes = extract_gecko_codes(&file_content); // Extract all Gecko Codes

    if let Some(first_gecko_code) = GeckoCode::from_str(&file_content) {
        // Since we split by "\n$", the very first Gecko Code (if it starts at the beginning of the file) will not be detected.
        gecko_codes.insert(0, first_gecko_code); // So we handle the first Gecko Code separately here.
    }

    let json_output = serde_json::to_string_pretty(&gecko_codes) // Serialize the vector to JSON
        .expect("Failed to serialize to JSON");

    fs::write("outputGeckoCodeBlob.json", json_output) // Save the JSON to "outputGeckoCodeBlob.json"
        .expect("Unable to write to file");

    println!("Successfully saved Gecko Codes to outputGeckoCodeBlob.json");
}
