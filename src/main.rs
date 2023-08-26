extern crate rayon;
extern crate regex;
use rayon::prelude::*;
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
    // etc.
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GeckoCode {
    header: String,
    version: Option<GameVersion>,
    authors: Option<Vec<String>>,
    description: Option<Vec<String>>,
    hex_lines: Vec<String>,
    hex_words: Vec<String>,
    deprecated: bool,
    addresses: Option<Vec<String>>,
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

        fn extract_hex_words(hex_line: &str) -> Vec<String> {
            let bytecode_pattern = Regex::new(r"^[\dA-Fa-fXxYyZz/?]{8}$").unwrap();
            hex_line
                .split_whitespace()
                .filter_map(|s| {
                    if bytecode_pattern.is_match(s) {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        }

        fn extract_opcode_and_address(hex_word: &str) -> Option<String> {
            let opcode = &hex_word[0..2];

            match opcode {
                "04" => {
                    // Logic specific to opcode "04"
                    Some(hex_word[2..].to_string())
                }
                "05" => {
                    // Logic specific to opcode "05"
                    Some(hex_word[2..].to_string())
                }
                "C2" => {
                    // Logic specific to opcode "C2"
                    Some(hex_word[2..].to_string())
                }
                "C3" => {
                    // Logic specific to opcode "C3"
                    Some(hex_word[2..].to_string())
                }
                _ => None, // No matching opcode
            }
        }

        let mut lines = s.lines();

        let mut gecko = GeckoCode {
            header: String::new(),
            version: None,
            authors: None,
            description: Some(Vec::new()),
            hex_lines: Vec::new(),
            hex_words: Vec::new(),
            deprecated: false,
            addresses: Some(Vec::new()),
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
            } else if Regex::new(r"^[\dA-Za-z]{8}\s?[\dA-Za-z\?]{8}")
                .unwrap()
                .is_match(line)
            {
                let trimmed_line = line.trim().to_string();
                gecko.hex_lines.push(trimmed_line.clone());

                let words = extract_hex_words(&trimmed_line);
                for word in &words {
                    gecko.hex_words.push(word.to_string());

                    if let Some(address) = extract_opcode_and_address(word) {
                        gecko.addresses.as_mut().unwrap().push(address);
                    }
                }
            }
        }

        if gecko.header.is_empty() || gecko.hex_lines.is_empty() {
            None
        } else {
            Some(gecko)
        }
    }
}

fn extract_gecko_codes(input: &str) -> Vec<GeckoCode> {
    let blocks: Vec<&str> = input.split("\n$").collect();

    println!("Total blocks found: {}", blocks.len());
    for (index, block) in blocks.iter().enumerate() {
        println!("Block {}: {}", index, block);
    }

    let mut gecko_codes: Vec<GeckoCode> = vec![];

    // Handle the very first block, which might not start with "$"
    if let Some(first_block) = blocks.first() {
        if let Some(first_gecko_code) = GeckoCode::from_str(first_block) {
            println!("First gecko code header: {:?}", first_gecko_code.header);
            gecko_codes.push(first_gecko_code);
        } else {
            println!("Failed to create gecko code for the first block");
        }
    }

    // Process the rest in parallel
    let remaining_gecko_codes = blocks
        .par_iter()
        .skip(1) // Skip the first block as it has been processed
        .filter_map(|&block| {
            let block_with_prefix = format!("${}", block); // Prefixing with "$"
            let code = GeckoCode::from_str(&block_with_prefix);
            if let Some(ref gecko_code) = code {
                println!("Parsed gecko code header: {:?}", gecko_code.header);
            } else {
                println!(
                    "Failed to create gecko code for block:\n{}",
                    block_with_prefix
                );
            }
            code
        })
        .collect::<Vec<GeckoCode>>();

    println!(
        "Number of GeckoCodes after processing: {}",
        remaining_gecko_codes.len()
    );

    gecko_codes.extend(remaining_gecko_codes);
    gecko_codes
}

#[derive(Debug, PartialEq)]
pub enum HexAddress {
    Address(String),
}

impl HexAddress {
    pub fn new(s: &str) -> Option<Self> {
        if HexAddress::is_valid(s) {
            Some(HexAddress::Address(s.to_string()))
        } else {
            None
        }
    }

    fn is_valid(s: &str) -> bool {
        let re = Regex::new(r"^[\dA-Fa-f]{6}$").unwrap();
        re.is_match(s)
    }
}

fn main() {
    env::set_var("RUST_BACKTRACE", "full");

    let file_path = Path::new("geckoCodeWikiPage.md");
    let file_content = fs::read_to_string(&file_path).expect("Unable to read file");

    let gecko_codes = extract_gecko_codes(&file_content);

    let json_output =
        serde_json::to_string_pretty(&gecko_codes).expect("Failed to serialize to JSON");

    fs::write("RawUnfilteredGeckoCodes.json", json_output).expect("Unable to write to file");

    println!("Successfully saved Gecko Codes to RawUnfilteredGeckoCodes.json");
}
