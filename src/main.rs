extern crate rayon;
extern crate regex;
use rayon::prelude::*;
use regex::Regex;
use reqwest; // For making HTTP requests
use scraper::{Html, Selector}; // For parsing HTML
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::Path,
};
use tokio;

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
            let header_pattern: Regex = Regex::new(r"^\$(.*)").unwrap();
            return header_pattern
                .captures(line)
                .map(|caps| caps[1].trim().to_string());
        }

        fn parse_authors(line: &str) -> Option<Vec<String>> {
            let author_pattern: Regex = Regex::new(r"\[([^]]+)\]").unwrap();
            return author_pattern
                .captures(line)
                .map(|caps| caps[1].split(',').map(|a| a.trim().to_string()).collect());
        }

        fn extract_hex_words(hex_line: &str) -> Vec<String> {
            let bytecode_pattern: Regex = Regex::new(r"^[\dA-Fa-fXxYyZz/?]{8}$").unwrap();
            return hex_line
                .split_whitespace()
                .filter_map(|s| {
                    if bytecode_pattern.is_match(s) {
                        Some(s.to_uppercase())
                    } else {
                        None
                    }
                })
                .collect();
        }

        fn extract_opcode_and_address(hex_words: &[&str], index: usize) -> Option<Vec<String>> {
            let opcode: &str = &hex_words[index][0..2];

            match opcode {
                "00" | "01" | // Direct RAM Writes: 8 bit Write & Fill
                "02" | "03" | // Direct RAM Writes: 16 bit Write & Fill
                "04" | "05" | // Direct RAM Writes: 32 bits Write
                "06" | "07" | // Direct RAM Writes: String Write (Patch Code)
                "08" | "09" | // Direct RAM Writes: Slider/Multi Skip (Serial)
                "20" | "21" | // If Codes: 32 bits (endif, then) If equal
                "22" | "23" | // If Codes: 32 bits (endif, then) If not equal
                "24" | "25" | // If Codes: 32 bits (endif, then) If greater than (unsigned)
                "26" | "27" | // If Codes: 32 bits (endif, then) If lower than (unsigned)
                "28" | "29" | // If Codes: 16 bits (endif, then) If equal
                "2A" | "2B" | // If Codes: 16 bits (endif, then) If not equal
                "2C" | "2D" | // If Codes: 16 bits (endif, then) If greater than
                "2E" | "2F" | // If Codes: 16 bits (endif, then) If lower than
                "A0" | "A1" | // Gecko Register: 16 bits (endif, then) If equal
                "A2" | "A3" | // Gecko Register: 16 bits (endif, then) If not equal
                "A4" | "A5" | // Gecko Register: 16 bits (endif, then) If greater
                "A6" | "A7" | // Gecko Register: 16 bits (endif, then) If lower
                "C2" | "C3" | // ASM Codes: Insert ASM
                "C6" | "C7" | // ASM Codes: Create a branch
                "F2" | "F3"   // Gecko 1.8+ Only: Insert ASM With 16 bit XOR Checksum
                => {
                    let mut base_mem_address: i64 =
                        i64::from_str_radix(&hex_words[index][2..], 16).ok()?;

                    // Account for if the memory address overflows into the OpCode hex
                    if opcode == "01" || 
                    opcode == "03" ||
                    opcode == "05" ||
                    opcode == "07" ||
                    opcode == "09" ||
                    opcode == "21" ||
                    opcode == "23" ||
                    opcode == "25" ||
                    opcode == "27" ||
                    opcode == "29" ||
                    opcode == "2B" ||
                    opcode == "2D" ||
                    opcode == "2F" ||
                    opcode == "A1" ||
                    opcode == "A3" ||
                    opcode == "A5" ||
                    opcode == "A7" ||
                    opcode == "C3" ||
                    opcode == "C7" || 
                    opcode == "F3" {
                        base_mem_address += 0x1000000;
                    }

                    Some(vec![format!("0x{:08X}", base_mem_address)])
                }
                _ => None, // No matching opcode
            }
        }

        let mut lines: std::str::Lines<'_> = s.lines();

        let mut gecko: GeckoCode = GeckoCode {
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
                let trimmed_line: String = line.trim().to_string();
                gecko.hex_lines.push(trimmed_line.clone());

                let words: Vec<String> = extract_hex_words(&trimmed_line);
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
            return None;
        } else {
            return Some(gecko);
        }
    }
}

fn extract_and_save_whole_gecko_codes(file_content: &str, raw_path: &Path, filtered_path: &Path) {
    let normalized_content: String = file_content.replace("\r\n", "\n"); // Normalize line endings first
    let content_to_split: String = if normalized_content.trim_start().starts_with("$") {
        // Check if the content starts with a code block and adjust accordingly
        format!("\n{}", normalized_content) // Prepend a newline for consistent processing
    } else {
        normalized_content.clone()
    };
    let blocks: Vec<&str> = content_to_split.split("\n$").collect(); // Now perform the split, ensuring the content lives long enough
    let blocks: Vec<&str> = if !normalized_content.trim_start().starts_with("$") {
        // If skipping non-code text before the first "$", adjust the blocks vector as needed
        blocks.into_iter().skip(1).collect()
    } else {
        blocks
    };
    let mut whole_gecko_codes: String = String::new();

    for block in blocks.iter() {
        if let Some(end_index) = block.find("\n&lt;/pre&gt;") {
            let formatted_block: String = format!("${}\n\n", &block[..end_index]);
            whole_gecko_codes.push_str(&formatted_block);
        } else if !block.trim().is_empty() {
            // Handle blocks without "\n&lt;/pre&gt;"
            let formatted_block: String = format!("${}\n\n", block.trim_end());
            whole_gecko_codes.push_str(&formatted_block);
        }
    }

    whole_gecko_codes = whole_gecko_codes
        .replace("&amp;", "&")
        .replace("\n&lt;/div&gt;\n", "")
        .replace("\n\n\n", "\n\n")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&cent;", "¢")
        .replace("&pound;", "£")
        .replace("&yen;", "¥")
        .replace("&euro;", "€")
        .replace("&copy;", "©")
        .replace("&reg;", "®");

    fs::write(raw_path, &whole_gecko_codes)
        .expect("Unable to write to `path_raw_whole_gecko_codes`.");

    println!("Successfully saved whole Gecko Codes to {:?}", raw_path);

    // deduplication logic
    whole_gecko_codes = format!("\n{}", whole_gecko_codes);
    let mut block_set: HashSet<&str> = HashSet::new();
    let mut unique_blocks: String = String::new();
    let mut is_first_block: bool = true;

    for block in whole_gecko_codes.split("\n$") {
        let trimmed_block: &str = block.trim();
        if !trimmed_block.is_empty() && !block_set.contains(trimmed_block) {
            block_set.insert(trimmed_block);
            if !is_first_block {
                unique_blocks.push_str("\n$");
            } else {
                is_first_block = false;
            }
            unique_blocks.push_str(trimmed_block);
            unique_blocks.push_str("\n");
        }
    }

    unique_blocks = format!("${}", unique_blocks);

    fs::write(filtered_path, unique_blocks.trim_end())
        .expect("Unable to write deduplicated blocks to `path_filtered_whole_gecko_codes`.");

    println!(
        "Successfully removed duplicates and saved to {:?}",
        filtered_path
    );
}

fn extract_and_destructure_gecko_codes(input: &str) -> Vec<GeckoCode> {
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
    let remaining_gecko_codes: Vec<GeckoCode> = blocks
        .par_iter()
        .skip(1) // Skip the first block as it has been processed
        .filter_map(|&block| {
            let block_with_prefix: String = format!("${}", block); // Prefixing with "$"
            let code: Option<GeckoCode> = GeckoCode::from_str(&block_with_prefix);
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

    
    gecko_codes.extend(remaining_gecko_codes);

    println!(
        "Number of GeckoCodes after processing: {}",
        gecko_codes.len()
    );
    return gecko_codes;
}

#[derive(Debug, PartialEq)]
pub enum HexAddress {
    Address(String),
}

impl HexAddress {
    pub fn new(s: &str) -> Option<Self> {
        if HexAddress::is_valid(s) {
            return Some(HexAddress::Address(s.to_string()));
        } else {
            return None;
        }
    }

    fn is_valid(s: &str) -> bool {
        let hex_address_pattern: Regex = Regex::new(r"^[\dA-Fa-f]{6}$").unwrap();
        return hex_address_pattern.is_match(s);
    }
}

fn parse_duplicate_addresses_md(file_content: &str) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let lines: Vec<&str> = file_content.lines().collect();
    let mut current_address: String = String::new();

    for line in lines {
        if line.starts_with("## Duplicate address:") {
            current_address = line
                .trim_start_matches("## Duplicate address: ")
                .to_string();
        } else if line.starts_with("- Found in code:") {
            let code_name: String = line.trim_start_matches("- Found in code: ").to_string();
            result
                .entry(current_address.clone())
                .or_insert_with(Vec::new)
                .push(code_name);
        }
    }

    return result;
}

fn group_by_code_headers(
    address_map: HashMap<String, Vec<String>>,
) -> HashMap<Vec<String>, Vec<String>> {
    let mut result: HashMap<Vec<String>, Vec<String>> = HashMap::new();

    for (address, codes) in address_map.iter() {
        let codes_sorted: Vec<String> = {
            let mut sorted_codes = codes.clone();
            sorted_codes.sort();
            sorted_codes
        };

        result
            .entry(codes_sorted)
            .or_insert_with(Vec::new)
            .push(address.clone());
    }

    return result;
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env::set_var("RUST_BACKTRACE", "full");

    let mut input: String = String::new();

    fs::create_dir_all("custom/")?;
    fs::create_dir_all("wiki")?;

    let file_content: String;
    let path_raw_whole_gecko_codes: &Path;
    let path_filtered_whole_gecko_codes: &Path;
    let path_raw_destructured_gecko_codes: &Path;
    let path_duplicated_addresses: &Path;
    let path_consolidated_addresses: &Path;

    println!("Type 'custom' to parse Gecko Codes from the file \"PLACE_GECKO_CODES_HERE.txt\", or 'wiki' to fetch the Hoard from the <https://wiki.supercombo.gg/w/SSBM/Gecko_Codes> web page:");
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let choice: &str = input.trim();
    match choice {
        "custom" => {
            file_content = fs::read_to_string("PLACE_GECKO_CODES_HERE.txt")
                .expect("Unable to read file \"PLACE_GECKO_CODES_HERE.txt\".");

            path_raw_whole_gecko_codes = Path::new("custom/RawWholeGeckoCodes.txt");
            path_filtered_whole_gecko_codes = Path::new("custom/FilteredWholeGeckoCodes.txt");
            path_raw_destructured_gecko_codes = Path::new("custom/RawDestructuredGeckoCodes.json");
            path_duplicated_addresses = Path::new("custom/DuplicatedAddresses.md");
            path_consolidated_addresses = Path::new("custom/ConsolidatedAddresses.md");
        }
        "wiki" => {
            let url: &str =
                "https://wiki.supercombo.gg/index.php?title=SSBM/Gecko_Codes&action=edit";
            let response: reqwest::Response =
                reqwest::get(url).await.expect("Failed to fetch the page");
            let body: String = response.text().await.expect("Failed to get response text");
            let document: Html = Html::parse_document(&body);
            let selector: Selector =
                Selector::parse("#wpTextbox1").expect("Failed to parse selector");
            let textarea_element: scraper::ElementRef<'_> = document
                .select(&selector)
                .next()
                .expect("Textarea element not found");
            file_content = textarea_element.inner_html();

            path_raw_whole_gecko_codes = Path::new("wiki/RawWholeGeckoCodes.txt");
            path_filtered_whole_gecko_codes = Path::new("wiki/FilteredWholeGeckoCodes.txt");
            path_raw_destructured_gecko_codes = Path::new("wiki/RawDestructuredGeckoCodes.json");
            path_duplicated_addresses = Path::new("wiki/DuplicatedAddresses.md");
            path_consolidated_addresses = Path::new("wiki/ConsolidatedAddresses.md");
        }
        _ => {
            panic!("Invalid option. Please type 'custom' or 'wiki' next time. Shutting down...");
        }
    }

    extract_and_save_whole_gecko_codes(
        &file_content,
        path_raw_whole_gecko_codes,
        path_filtered_whole_gecko_codes,
    );

    let filtered_codes: String = fs::read_to_string(&path_filtered_whole_gecko_codes)
        .expect("Failed to read path_filtered_whole_gecko_codes");
    let gecko_codes: Vec<GeckoCode> = extract_and_destructure_gecko_codes(&filtered_codes);
    let json_output: String =
        serde_json::to_string_pretty(&gecko_codes).expect("Failed to serialize to JSON");

    fs::write(&path_raw_destructured_gecko_codes, json_output)
        .expect("Unable to write to `path_raw_destructured_gecko_codes`");

    println!(
        "Successfully saved Gecko Codes to {:?}",
        path_raw_destructured_gecko_codes
    );

    // Deserialize the stored JSON file
    let json_content: String =
        fs::read_to_string(&path_raw_destructured_gecko_codes).expect("Unable to read JSON file");
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
    address_map.retain(|_, headers: &mut Vec<String>| {
        headers.sort(); // Sort the headers for consistent comparison
        headers.dedup(); // Remove duplicate headers

        // Only retain the address if there's more than one unique header
        headers.len() > 1
    });

    // Sort the addresses
    let mut sorted_addresses: Vec<String> = address_map.keys().cloned().collect();
    sorted_addresses.sort();

    // Store results in a markdown formatted string
    let mut md_content: String = String::new();
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
    fs::write(&path_duplicated_addresses, md_content)
        .expect("Unable to write to `path_duplicated_addresses`.");

    println!(
        "Successfully saved sorted and cleaned-up duplicate addresses to {:?}",
        path_duplicated_addresses
    );

    // Parse DuplicateAddresses.md and consolidate entries
    let md_content: String = fs::read_to_string(&path_duplicated_addresses)
        .expect("Unable to read `path_dupliucated_addresses`");
    let parsed_data: HashMap<String, Vec<String>> = parse_duplicate_addresses_md(&md_content);
    let grouped_data: HashMap<Vec<String>, Vec<String>> = group_by_code_headers(parsed_data);

    let mut new_md_content: String = String::new();
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

    fs::write(&path_consolidated_addresses, new_md_content)
        .expect("Unable to write to `path_consolidated_addresses`");

    println!(
        "Successfully saved consolidated addresses to {:?}",
        path_consolidated_addresses
    );

    return Ok(());
}
