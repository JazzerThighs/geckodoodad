extern crate rayon;
extern crate regex;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
// use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum Category {
    Gameplay,
    Aesthetics,
    // etc.
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GeckoCode {
    header: String,
    version: String,
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
                        Some(s.to_uppercase())
                    } else {
                        None
                    }
                })
                .collect()
        }

        fn extract_opcode_and_address(hex_words: &[&str], index: usize) -> Option<Vec<String>> {
            let opcode = &hex_words[index][0..2];

            match opcode {
                "04" | "05" | "C2" | "C3" => {
                    let mut base_mem_address =
                        i64::from_str_radix(&hex_words[index][2..], 16).ok()?;

                    // Account for if the memory address overflows into the OpCode hex
                    if opcode == "05" || opcode == "C3" {
                        base_mem_address += 0x1000000;
                    }

                    Some(vec![format!("0x{:08X}", base_mem_address)])
                }
                _ => None, // No matching opcode
            }
        }

        let mut lines = s.lines();

        let mut gecko = GeckoCode {
            header: String::new(),
            version: String::from(""),
            authors: None,
            description: Some(Vec::new()),
            hex_lines: Vec::new(),
            hex_words: Vec::new(),
            deprecated: false,
            addresses: Some(Vec::new()),
            categories: Some(Vec::new()),
        };

        if let Some(header) = lines.next().and_then(|line| parse_header(line)) {
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
                for (index, word) in words.iter().enumerate() {
                    gecko.hex_words.push(word.to_string());

                    let str_slice: Vec<&str> = words.iter().map(AsRef::as_ref).collect();
                    if let Some(addresses) = extract_opcode_and_address(&str_slice[..], index) {
                        for address in addresses {
                            gecko.addresses.as_mut().unwrap().push(address);
                        }
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

fn parse_duplicate_addresses_md(file_content: &str) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();
    let lines: Vec<&str> = file_content.lines().collect();
    let mut current_address = String::new();

    for line in lines {
        if line.starts_with("## Duplicate address:") {
            current_address = line
                .trim_start_matches("## Duplicate address: ")
                .to_string();
        } else if line.starts_with("- Found in code:") {
            let code_name = line.trim_start_matches("- Found in code: ").to_string();
            result
                .entry(current_address.clone())
                .or_insert_with(Vec::new)
                .push(code_name);
        }
    }

    result
}

fn group_by_code_headers(
    address_map: HashMap<String, Vec<String>>,
) -> HashMap<Vec<String>, Vec<String>> {
    let mut result: HashMap<Vec<String>, Vec<String>> = HashMap::new();

    for (address, codes) in address_map.iter() {
        let codes_sorted = {
            let mut sorted_codes = codes.clone();
            sorted_codes.sort();
            sorted_codes
        };

        result
            .entry(codes_sorted)
            .or_insert_with(Vec::new)
            .push(address.clone());
    }

    result
}

fn main() {
    env::set_var("RUST_BACKTRACE", "full");

    let file_path = Path::new("geckoCodeWikiPage.md"); //NOTE: When updating the raw .md file from the Wiki, use Shift+Tab on the whole document to remove the leading whitespace from every line of text.
    let file_content = fs::read_to_string(&file_path).expect("Unable to read file");

    let gecko_codes = extract_gecko_codes(&file_content);

    let json_output =
        serde_json::to_string_pretty(&gecko_codes).expect("Failed to serialize to JSON");

    fs::write("RawUnfilteredGeckoCodes.json", json_output).expect("Unable to write to file");

    println!("Successfully saved Gecko Codes to RawUnfilteredGeckoCodes.json");

    // Deserialize the stored JSON file
    let json_content =
        fs::read_to_string("RawUnfilteredGeckoCodes.json").expect("Unable to read JSON file");
    let deserialized_gecko_codes: Vec<GeckoCode> =
        serde_json::from_str(&json_content).expect("Failed to deserialize JSON");

    // Identify duplicate addresses
    let mut address_map: HashMap<String, Vec<String>> = HashMap::new();

    for gecko_code in &deserialized_gecko_codes {
        if let Some(addresses) = &gecko_code.addresses {
            for address in addresses {
                address_map
                    .entry(address.clone())
                    .or_insert_with(Vec::new)
                    .push(gecko_code.header.clone());
            }
        }
    }

    // Post-process the address_map
    address_map.retain(|_, headers| {
        headers.sort(); // Sort the headers for consistent comparison
        headers.dedup(); // Remove duplicate headers

        // Only retain the address if there's more than one unique header
        headers.len() > 1
    });

    // Sort the addresses
    let mut sorted_addresses: Vec<String> = address_map.keys().cloned().collect();
    sorted_addresses.sort();

    // Store results in a markdown formatted string
    let mut md_content = String::new();
    for address in sorted_addresses {
        if let Some(code_headers) = address_map.get(&address) {
            md_content += &format!("## Duplicate address: {}\n", address);
            for header in code_headers {
                md_content += &format!("- Found in code: {}\n", header);
            }
            md_content += "\n"; // Add an extra newline for spacing
        }
    }

    // Save the results to a markdown file
    fs::write("DuplicateAddresses.md", md_content)
        .expect("Unable to write to DuplicateAddresses.md");

    println!(
        "Successfully saved sorted and cleaned-up duplicate addresses to DuplicateAddresses.md"
    );

    // Parse DuplicateAddresses.md and consolidate entries
    let md_content =
        fs::read_to_string("DuplicateAddresses.md").expect("Unable to read DuplicateAddresses.md");
    let parsed_data = parse_duplicate_addresses_md(&md_content);
    let grouped_data = group_by_code_headers(parsed_data);

    let mut new_md_content = String::new();
    for (code_names, addresses) in grouped_data.iter() {
        new_md_content += &format!("## Codes:\n");
        for code_name in code_names {
            new_md_content += &format!("- {}\n", code_name);
        }
        new_md_content += "### Shared addresses:\n";
        for address in addresses {
            new_md_content += &format!("- {}\n", address);
        }
        new_md_content += "\n";
    }

    fs::write("ConsolidatedAddresses.md", new_md_content)
        .expect("Unable to write to ConsolidatedAddresses.md");

    println!("Successfully saved consolidated addresses to ConsolidatedAddresses.md");
}
