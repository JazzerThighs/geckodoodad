extern crate regex;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
// use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum GameVersion {
    NTSC(f64),
    KOR,
    PAL,
    Other, // to handle unexpected versions
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum Category {
    Gameplay,
    Aesthetics,
    Features,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GeckoCode {
    header: String,
    version: Option<GameVersion>,
    authors: Option<Vec<String>>,
    description: Option<Vec<String>>,
    hex: Vec<String>,
    deprecated: bool,
    overwrite: Option<Vec<String>>,
    injection: Option<Vec<String>>,
    categories: Option<Vec<String>>,
}

impl GeckoCode {
    pub fn from_str(s: &str) -> Option<GeckoCode> {
        // Helper functions for parsing various sections of the Gecko Code.
        fn parse_header(line: &str) -> Option<String> {
            let re = Regex::new(r"^\$(.*)").unwrap();
            re.captures(line).map(|caps| caps[1].trim().to_string())
        }

        fn parse_version(line: &str) -> Option<GameVersion> {
            let re = Regex::new(r"\(([^)]+)\)").unwrap();
            re.captures(line).and_then(|caps| match &caps[1] {
                "1.0" | "1.00" | "v1.0" | "v1.00" => Some(GameVersion::NTSC(1.0)),
                "1.01" | "v1.01" => Some(GameVersion::NTSC(1.01)),
                "1.02" | "v1.02" => Some(GameVersion::NTSC(1.02)),
                "KOR" => Some(GameVersion::KOR),
                "PAL" => Some(GameVersion::PAL),
                "20XX" | "20XXHP" | "Beyond" | "UPTM" | "UP" | "1.03" | "v1.03" | "Silly Melee" => {
                    Some(GameVersion::Other)
                }
                _ => None,
            })
        }

        fn parse_authors(line: &str) -> Option<Vec<String>> {
            let re = Regex::new(r"\[([^]]+)\]").unwrap();
            re.captures(line)
                .map(|caps| caps[1].split(',').map(|a| a.trim().to_string()).collect())
        }

        let mut lines = s.lines();

        let mut gecko = GeckoCode {
            header: String::new(),
            version: None,
            authors: None,
            description: Some(Vec::new()),
            hex: Vec::new(),
            deprecated: false,
            overwrite: Some(Vec::new()),
            injection: Some(Vec::new()),
            categories: Some(Vec::new()),
        };

        if let Some(header) = lines.next().and_then(|line| parse_header(line)) {
            if let Some(version) = parse_version(&header) {
                gecko.version = Some(version);
            }
            if let Some(authors) = parse_authors(&header) {
                gecko.authors = Some(authors);
            }
            gecko.header = header;
        } else {
            return None;
        }

        for line in lines {
            if line.starts_with('*') {
                if gecko.description.is_none() {
                    gecko.description = Some(Vec::new());
                }
                gecko
                    .description
                    .as_mut()
                    .unwrap()
                    .push(line[1..].trim().to_string());
            } else if Regex::new(r"^[\dA-Za-z]{8}\s?[\dA-Za-z]{8}")
                .unwrap()
                .is_match(line)
            {
                gecko.hex.push(line.trim().to_string());
            }
        }

        if gecko.header.is_empty() || gecko.hex.is_empty() {
            None
        } else {
            Some(gecko)
        }
    }
}

fn extract_gecko_codes(input: &str) -> Vec<GeckoCode> {
    let mut gecko_codes = Vec::new();
    let mut current_code_block = String::new();
    let mut capturing = false;

    for line in input.lines() {
        // Start capturing when a line starts with "$"
        if line.starts_with("$") {
            capturing = true;
        }

        // If currently capturing a Gecko Code block
        if capturing {
            current_code_block.push_str(line);
            current_code_block.push('\n');

            // If line is not a hex line, end capturing
            if !Regex::new(r"^[\dA-Fa-fxyXY]{8} [\dA-Fa-fxyXY]{8}")
                .unwrap()
                .is_match(line)
                && !line.starts_with("*")
                && !line.starts_with("$")
            {
                capturing = false;
                if let Some(gecko_code) = GeckoCode::from_str(&current_code_block) {
                    println!("{:?} added;", gecko_code.header);
                    gecko_codes.push(gecko_code);
                }
                current_code_block.clear();
            }
        }
    }

    // Handle the case where the last Gecko Code goes till the end of the file
    if !current_code_block.is_empty() {
        if let Some(gecko_code) = GeckoCode::from_str(&current_code_block) {
            gecko_codes.push(gecko_code);
        }
    }

    gecko_codes
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct FilteredGeckoCode {
    header: String,
    version: Option<GameVersion>,
    authors: Option<Vec<String>>,
    description: Option<Vec<String>>,
    hex: Vec<[String; 2]>, // This will store the pair of hex strings
    deprecated: bool,
    overwrite: Option<Vec<String>>,
    injection: Option<Vec<String>>,
    categories: Option<Vec<String>>,
}

impl FilteredGeckoCode {
    pub fn from_gecko_code(gecko: &GeckoCode) -> Self {
        let hex_pairs = gecko
            .hex
            .iter()
            .map(|hex_string| {
                let mut split = hex_string
                    .split_whitespace()
                    .map(String::from)
                    .collect::<Vec<_>>();
                [split.remove(0), split.remove(0)]
            })
            .collect();

        FilteredGeckoCode {
            header: gecko.header.clone(),
            version: gecko.version.clone(),
            authors: gecko.authors.clone(),
            description: gecko.description.clone(),
            hex: hex_pairs,
            deprecated: gecko.deprecated,
            overwrite: gecko.overwrite.clone(),
            injection: gecko.injection.clone(),
            categories: gecko.categories.clone(),
        }
    }
}

fn main() {
    env::set_var("RUST_BACKTRACE", "full"); // this method needs to be inside main() method

    let file_path = Path::new("geckoCodeWikiPage.md"); // Read the markdown file
    let file_content = fs::read_to_string(&file_path).expect("Unable to read file");

    let mut gecko_codes = extract_gecko_codes(&file_content); // Extract all Gecko Codes

    if let Some(first_gecko_code) = GeckoCode::from_str(&file_content) {
        // Since we split by "\n$", the very first Gecko Code (if it starts at the beginning of the file) will not be detected.
        gecko_codes.insert(0, first_gecko_code); // So we handle the first Gecko Code separately here.
    }

    let json_output = serde_json::to_string_pretty(&gecko_codes) // Serialize the vector to JSON
        .expect("Failed to serialize to JSON");

    fs::write("RawUnfilteredGeckoCodes.json", &json_output) // Save the JSON to "outputGeckoCodeBlob.json"
        .expect("Unable to write to file");

    println!("Successfully saved Gecko Codes to RawUnfilteredGeckoCodes.json");

    let raw_data: Vec<GeckoCode> = serde_json::from_str(&json_output).unwrap_or_default();

    let mut once_filtered_gecko_codes: Vec<FilteredGeckoCode> = Vec::new();

    for gecko_code in raw_data {
        let mut filtered_gecko_code = FilteredGeckoCode::from_gecko_code(&gecko_code);

        // Update for each hex code string in gecko_code
        for hex_code in &gecko_code.hex {
            // Split the hex string by whitespace and then rejoin the first two parts (to remove comments and extra spaces)
            let split_hex: Vec<&str> = hex_code.split_whitespace().collect();

            if split_hex.len() >= 2 {
                let truncated_hex = format!("{} {}", split_hex[0], split_hex[1]);
                filtered_gecko_code
                    .hex
                    .push([split_hex[0].to_string(), split_hex[1].to_string()]);

                // Classify hex codes
                if truncated_hex.starts_with("C2") || truncated_hex.starts_with("C3") {
                    filtered_gecko_code
                        .injection
                        .as_mut()
                        .unwrap()
                        .push(truncated_hex.clone());
                }
                if truncated_hex.starts_with("04") || truncated_hex.starts_with("05") {
                    filtered_gecko_code
                        .overwrite
                        .as_mut()
                        .unwrap()
                        .push(truncated_hex.clone());
                }
            }
        }
        once_filtered_gecko_codes.push(filtered_gecko_code);
    }

    let filtered_json_output = serde_json::to_string_pretty(&once_filtered_gecko_codes)
        .expect("Failed to serialize to JSON");

    fs::write("OnceFilteredGeckoCodes.json", &filtered_json_output)
        .expect("Unable to write to file");

    println!("Successfully saved filtered Gecko Codes to OnceFilteredGeckoCodes.json");
}
