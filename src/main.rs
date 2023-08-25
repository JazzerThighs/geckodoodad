extern crate regex;

use regex::Regex;
use serde::Serialize;
use serde_json;
// use std::collections::HashMap;
use std::{fs, env};
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
            let re = Regex::new(r"^\(([^)]+)\)").unwrap();
            re.captures(line).and_then(|caps| match &caps[1] {
                "1.0" | "1.00" => Some(GameVersion::NTSC(1.0)),
                "1.01" => Some(GameVersion::NTSC(1.01)),
                "1.02" => Some(GameVersion::NTSC(1.02)),
                "KOR" => Some(GameVersion::KOR),
                "PAL" => Some(GameVersion::PAL),
                _ => None,
            })
        }

        fn parse_authors(line: &str) -> Option<Vec<String>> {
            let re = Regex::new(r"^\[([^]]+)\]").unwrap();
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
            gecko.header = header;
        } else {
            return None;
        }

        for line in lines {
            if let Some(version) = parse_version(line) {
                gecko.version = Some(version);
            } else if let Some(authors) = parse_authors(line) {
                gecko.authors = Some(authors);
            } else if line.starts_with('*') {
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

    fs::write("outputGeckoCodeBlob.json", json_output) // Save the JSON to "outputGeckoCodeBlob.json"
        .expect("Unable to write to file");

    println!("Successfully saved Gecko Codes to outputGeckoCodeBlob.json");
}
