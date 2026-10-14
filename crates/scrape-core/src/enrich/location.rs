use super::{DescriptionBlock, DescriptionInline, JobDescription};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParsedLocation {
    pub raw: String,
    pub city: Option<String>,
    pub region_code: Option<String>, // ISO 3166-2 e.g. "US-CA", "CA-ON"
    pub country_code: Option<String>, // ISO 3166-1 alpha-2 e.g. "US", "CA", "GB"
    pub is_remote: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationSectionExtraction {
    pub section_found: bool,
    pub locations: Vec<ParsedLocation>,
}

pub fn has_geographic_value(locations: &[ParsedLocation]) -> bool {
    locations.iter().any(|location| {
        location.city.is_some() || location.region_code.is_some() || location.country_code.is_some()
    })
}

struct StateEntry {
    name: &'static str,
    abbr: &'static str,
    region_code: &'static str,
    country_code: &'static str,
}

static US_STATES: &[StateEntry] = &[
    StateEntry {
        name: "ALABAMA",
        abbr: "AL",
        region_code: "US-AL",
        country_code: "US",
    },
    StateEntry {
        name: "ALASKA",
        abbr: "AK",
        region_code: "US-AK",
        country_code: "US",
    },
    StateEntry {
        name: "ARIZONA",
        abbr: "AZ",
        region_code: "US-AZ",
        country_code: "US",
    },
    StateEntry {
        name: "ARKANSAS",
        abbr: "AR",
        region_code: "US-AR",
        country_code: "US",
    },
    StateEntry {
        name: "CALIFORNIA",
        abbr: "CA",
        region_code: "US-CA",
        country_code: "US",
    },
    StateEntry {
        name: "COLORADO",
        abbr: "CO",
        region_code: "US-CO",
        country_code: "US",
    },
    StateEntry {
        name: "CONNECTICUT",
        abbr: "CT",
        region_code: "US-CT",
        country_code: "US",
    },
    StateEntry {
        name: "DELAWARE",
        abbr: "DE",
        region_code: "US-DE",
        country_code: "US",
    },
    StateEntry {
        name: "DISTRICT OF COLUMBIA",
        abbr: "DC",
        region_code: "US-DC",
        country_code: "US",
    },
    StateEntry {
        name: "FLORIDA",
        abbr: "FL",
        region_code: "US-FL",
        country_code: "US",
    },
    StateEntry {
        name: "GEORGIA",
        abbr: "GA",
        region_code: "US-GA",
        country_code: "US",
    },
    StateEntry {
        name: "HAWAII",
        abbr: "HI",
        region_code: "US-HI",
        country_code: "US",
    },
    StateEntry {
        name: "IDAHO",
        abbr: "ID",
        region_code: "US-ID",
        country_code: "US",
    },
    StateEntry {
        name: "ILLINOIS",
        abbr: "IL",
        region_code: "US-IL",
        country_code: "US",
    },
    StateEntry {
        name: "INDIANA",
        abbr: "IN",
        region_code: "US-IN",
        country_code: "US",
    },
    StateEntry {
        name: "IOWA",
        abbr: "IA",
        region_code: "US-IA",
        country_code: "US",
    },
    StateEntry {
        name: "KANSAS",
        abbr: "KS",
        region_code: "US-KS",
        country_code: "US",
    },
    StateEntry {
        name: "KENTUCKY",
        abbr: "KY",
        region_code: "US-KY",
        country_code: "US",
    },
    StateEntry {
        name: "LOUISIANA",
        abbr: "LA",
        region_code: "US-LA",
        country_code: "US",
    },
    StateEntry {
        name: "MAINE",
        abbr: "ME",
        region_code: "US-ME",
        country_code: "US",
    },
    StateEntry {
        name: "MARYLAND",
        abbr: "MD",
        region_code: "US-MD",
        country_code: "US",
    },
    StateEntry {
        name: "MASSACHUSETTS",
        abbr: "MA",
        region_code: "US-MA",
        country_code: "US",
    },
    StateEntry {
        name: "MICHIGAN",
        abbr: "MI",
        region_code: "US-MI",
        country_code: "US",
    },
    StateEntry {
        name: "MINNESOTA",
        abbr: "MN",
        region_code: "US-MN",
        country_code: "US",
    },
    StateEntry {
        name: "MISSISSIPPI",
        abbr: "MS",
        region_code: "US-MS",
        country_code: "US",
    },
    StateEntry {
        name: "MISSOURI",
        abbr: "MO",
        region_code: "US-MO",
        country_code: "US",
    },
    StateEntry {
        name: "MONTANA",
        abbr: "MT",
        region_code: "US-MT",
        country_code: "US",
    },
    StateEntry {
        name: "NEBRASKA",
        abbr: "NE",
        region_code: "US-NE",
        country_code: "US",
    },
    StateEntry {
        name: "NEVADA",
        abbr: "NV",
        region_code: "US-NV",
        country_code: "US",
    },
    StateEntry {
        name: "NEW HAMPSHIRE",
        abbr: "NH",
        region_code: "US-NH",
        country_code: "US",
    },
    StateEntry {
        name: "NEW JERSEY",
        abbr: "NJ",
        region_code: "US-NJ",
        country_code: "US",
    },
    StateEntry {
        name: "NEW MEXICO",
        abbr: "NM",
        region_code: "US-NM",
        country_code: "US",
    },
    StateEntry {
        name: "NEW YORK",
        abbr: "NY",
        region_code: "US-NY",
        country_code: "US",
    },
    StateEntry {
        name: "NORTH CAROLINA",
        abbr: "NC",
        region_code: "US-NC",
        country_code: "US",
    },
    StateEntry {
        name: "NORTH DAKOTA",
        abbr: "ND",
        region_code: "US-ND",
        country_code: "US",
    },
    StateEntry {
        name: "OHIO",
        abbr: "OH",
        region_code: "US-OH",
        country_code: "US",
    },
    StateEntry {
        name: "OKLAHOMA",
        abbr: "OK",
        region_code: "US-OK",
        country_code: "US",
    },
    StateEntry {
        name: "OREGON",
        abbr: "OR",
        region_code: "US-OR",
        country_code: "US",
    },
    StateEntry {
        name: "PENNSYLVANIA",
        abbr: "PA",
        region_code: "US-PA",
        country_code: "US",
    },
    StateEntry {
        name: "RHODE ISLAND",
        abbr: "RI",
        region_code: "US-RI",
        country_code: "US",
    },
    StateEntry {
        name: "SOUTH CAROLINA",
        abbr: "SC",
        region_code: "US-SC",
        country_code: "US",
    },
    StateEntry {
        name: "SOUTH DAKOTA",
        abbr: "SD",
        region_code: "US-SD",
        country_code: "US",
    },
    StateEntry {
        name: "TENNESSEE",
        abbr: "TN",
        region_code: "US-TN",
        country_code: "US",
    },
    StateEntry {
        name: "TEXAS",
        abbr: "TX",
        region_code: "US-TX",
        country_code: "US",
    },
    StateEntry {
        name: "UTAH",
        abbr: "UT",
        region_code: "US-UT",
        country_code: "US",
    },
    StateEntry {
        name: "VERMONT",
        abbr: "VT",
        region_code: "US-VT",
        country_code: "US",
    },
    StateEntry {
        name: "VIRGINIA",
        abbr: "VA",
        region_code: "US-VA",
        country_code: "US",
    },
    StateEntry {
        name: "WASHINGTON",
        abbr: "WA",
        region_code: "US-WA",
        country_code: "US",
    },
    StateEntry {
        name: "WEST VIRGINIA",
        abbr: "WV",
        region_code: "US-WV",
        country_code: "US",
    },
    StateEntry {
        name: "WISCONSIN",
        abbr: "WI",
        region_code: "US-WI",
        country_code: "US",
    },
    StateEntry {
        name: "WYOMING",
        abbr: "WY",
        region_code: "US-WY",
        country_code: "US",
    },
];

static CA_PROVINCES: &[StateEntry] = &[
    StateEntry {
        name: "ONTARIO",
        abbr: "ON",
        region_code: "CA-ON",
        country_code: "CA",
    },
    StateEntry {
        name: "BRITISH COLUMBIA",
        abbr: "BC",
        region_code: "CA-BC",
        country_code: "CA",
    },
    StateEntry {
        name: "QUEBEC",
        abbr: "QC",
        region_code: "CA-QC",
        country_code: "CA",
    },
    StateEntry {
        name: "ALBERTA",
        abbr: "AB",
        region_code: "CA-AB",
        country_code: "CA",
    },
    StateEntry {
        name: "MANITOBA",
        abbr: "MB",
        region_code: "CA-MB",
        country_code: "CA",
    },
    StateEntry {
        name: "SASKATCHEWAN",
        abbr: "SK",
        region_code: "CA-SK",
        country_code: "CA",
    },
    StateEntry {
        name: "NOVA SCOTIA",
        abbr: "NS",
        region_code: "CA-NS",
        country_code: "CA",
    },
];

struct CityEntry {
    name: &'static str,
    city: &'static str,
    region_code: &'static str,
    country_code: &'static str,
}

static TOP_CITIES: &[CityEntry] = &[
    CityEntry {
        name: "SAN FRANCISCO",
        city: "San Francisco",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "SAN JOSE",
        city: "San Jose",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "OAKLAND",
        city: "Oakland",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "LOS ANGELES",
        city: "Los Angeles",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "SAN DIEGO",
        city: "San Diego",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "PALO ALTO",
        city: "Palo Alto",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "MOUNTAIN VIEW",
        city: "Mountain View",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "SUNNYVALE",
        city: "Sunnyvale",
        region_code: "US-CA",
        country_code: "US",
    },
    CityEntry {
        name: "NEW YORK",
        city: "New York",
        region_code: "US-NY",
        country_code: "US",
    },
    CityEntry {
        name: "NYC",
        city: "New York",
        region_code: "US-NY",
        country_code: "US",
    },
    CityEntry {
        name: "BROOKLYN",
        city: "New York",
        region_code: "US-NY",
        country_code: "US",
    },
    CityEntry {
        name: "SEATTLE",
        city: "Seattle",
        region_code: "US-WA",
        country_code: "US",
    },
    CityEntry {
        name: "BELLEVUE",
        city: "Bellevue",
        region_code: "US-WA",
        country_code: "US",
    },
    CityEntry {
        name: "AUSTIN",
        city: "Austin",
        region_code: "US-TX",
        country_code: "US",
    },
    CityEntry {
        name: "DALLAS",
        city: "Dallas",
        region_code: "US-TX",
        country_code: "US",
    },
    CityEntry {
        name: "HOUSTON",
        city: "Houston",
        region_code: "US-TX",
        country_code: "US",
    },
    CityEntry {
        name: "BOSTON",
        city: "Boston",
        region_code: "US-MA",
        country_code: "US",
    },
    CityEntry {
        name: "CAMBRIDGE",
        city: "Cambridge",
        region_code: "US-MA",
        country_code: "US",
    },
    CityEntry {
        name: "CHICAGO",
        city: "Chicago",
        region_code: "US-IL",
        country_code: "US",
    },
    CityEntry {
        name: "DENVER",
        city: "Denver",
        region_code: "US-CO",
        country_code: "US",
    },
    CityEntry {
        name: "BOULDER",
        city: "Boulder",
        region_code: "US-CO",
        country_code: "US",
    },
    CityEntry {
        name: "ATLANTA",
        city: "Atlanta",
        region_code: "US-GA",
        country_code: "US",
    },
    CityEntry {
        name: "WASHINGTON",
        city: "Washington",
        region_code: "US-DC",
        country_code: "US",
    },
    CityEntry {
        name: "TORONTO",
        city: "Toronto",
        region_code: "CA-ON",
        country_code: "CA",
    },
    CityEntry {
        name: "VANCOUVER",
        city: "Vancouver",
        region_code: "CA-BC",
        country_code: "CA",
    },
    CityEntry {
        name: "MONTREAL",
        city: "Montreal",
        region_code: "CA-QC",
        country_code: "CA",
    },
    CityEntry {
        name: "LONDON",
        city: "London",
        region_code: "GB-ENG",
        country_code: "GB",
    },
    CityEntry {
        name: "BERLIN",
        city: "Berlin",
        region_code: "DE-BE",
        country_code: "DE",
    },
    CityEntry {
        name: "AMSTERDAM",
        city: "Amsterdam",
        region_code: "NL-NH",
        country_code: "NL",
    },
    CityEntry {
        name: "DUBLIN",
        city: "Dublin",
        region_code: "IE-D",
        country_code: "IE",
    },
];

pub fn parse(raw_input: &str) -> Vec<ParsedLocation> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // Split on compound delimiters: ";", "|", "/", " or "
    let parts: Vec<&str> = trimmed
        .split([';', '|', '/'])
        .flat_map(|p| p.split(" or "))
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    if parts.is_empty() {
        return vec![parse_single(trimmed)];
    }

    parts.into_iter().map(parse_single).collect()
}

/// Extract eligible geography only from explicitly labeled location sections
/// in a normalized job description. `raw` on each returned location retains
/// the section label and candidate as its evidence provenance.
pub fn from_description_sections(description: &JobDescription) -> LocationSectionExtraction {
    const MAX_VALUES: usize = 16;
    const MAX_VALUE_BYTES: usize = 256;

    let mut active_label: Option<String> = None;
    let mut locations = Vec::new();
    let mut section_found = false;

    for block in &description.blocks {
        match block {
            DescriptionBlock::Heading { children, .. } => {
                let text = inline_text(children);
                active_label = location_section_label(&text).map(str::to_owned);
                section_found |= active_label.is_some();
            }
            DescriptionBlock::Paragraph { children } => {
                let text = inline_text(children);
                if let Some((label, value)) = labeled_location_value(&text) {
                    section_found = true;
                    add_location_value(&mut locations, label, value, MAX_VALUES, MAX_VALUE_BYTES);
                    active_label = None;
                } else if let Some(label) = location_section_label(&text) {
                    active_label = Some(label.to_owned());
                    section_found = true;
                } else if let Some(label) = active_label.as_deref() {
                    add_location_value(&mut locations, label, &text, MAX_VALUES, MAX_VALUE_BYTES);
                }
            }
            DescriptionBlock::List { items, .. } => {
                if let Some(label) = active_label.as_deref() {
                    for item in items {
                        add_location_value(
                            &mut locations,
                            label,
                            &inline_text(item),
                            MAX_VALUES,
                            MAX_VALUE_BYTES,
                        );
                    }
                }
            }
            DescriptionBlock::Break => {}
        }
        if locations.len() >= MAX_VALUES {
            break;
        }
    }

    LocationSectionExtraction {
        section_found,
        locations,
    }
}

fn add_location_value(
    locations: &mut Vec<ParsedLocation>,
    label: &str,
    raw_value: &str,
    max_values: usize,
    max_value_bytes: usize,
) {
    if locations.len() >= max_values {
        return;
    }
    let value = raw_value
        .trim()
        .trim_matches(['•', '-', '–', '—', ':', ' ']);
    if value.is_empty() || value.len() > max_value_bytes {
        return;
    }
    let comma_parts: Vec<_> = value.split(',').map(str::trim).collect();
    let parsed = if comma_parts.len() > 1
        && comma_parts.iter().all(|part| {
            let parsed = parse(part);
            !parsed.is_empty()
                && parsed.iter().all(|location| {
                    location.country_code.is_some()
                        && location.region_code.is_none()
                        && location.city.is_none()
                })
        }) {
        comma_parts.iter().flat_map(|part| parse(part)).collect()
    } else {
        parse(value)
    };
    for mut location in parsed {
        if location.country_code.is_none()
            && location.region_code.is_none()
            && location.city.is_none()
        {
            continue;
        }
        location.raw = format!("{label}: {value}");
        if !locations.iter().any(|existing| {
            existing.city == location.city
                && existing.region_code == location.region_code
                && existing.country_code == location.country_code
                && existing.is_remote == location.is_remote
        }) {
            locations.push(location);
        }
    }
}

fn inline_text(nodes: &[DescriptionInline]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            DescriptionInline::Text { text: value } => text.push_str(value),
            DescriptionInline::Strong { children } | DescriptionInline::Emphasis { children } => {
                text.push_str(&inline_text(children));
            }
            DescriptionInline::Link { text: value, .. } => text.push_str(value),
        }
    }
    text
}

fn location_section_label(value: &str) -> Option<&'static str> {
    let label = value
        .trim()
        .trim_end_matches([':', '.', ' '])
        .to_ascii_lowercase();
    match label.as_str() {
        "available locations" | "available work locations" => Some("Available Locations"),
        "eligible locations" | "eligible countries" => Some("Eligible Locations"),
        "work location" | "work locations" => Some("Work Location"),
        "location" | "locations" => Some("Location"),
        _ => None,
    }
}

fn labeled_location_value(value: &str) -> Option<(&'static str, &str)> {
    let trimmed = value.trim();
    for label in [
        "available work locations",
        "available locations",
        "eligible locations",
        "eligible countries",
        "work locations",
        "work location",
        "locations",
        "location",
    ] {
        let Some(rest) = trimmed.get(label.len()..) else {
            continue;
        };
        if !trimmed[..label.len()].eq_ignore_ascii_case(label) {
            continue;
        }
        let rest = rest.trim_start();
        let rest = rest
            .strip_prefix(':')
            .or_else(|| rest.strip_prefix('—'))
            .or_else(|| rest.strip_prefix('–'))
            .or_else(|| rest.strip_prefix('-'))?;
        let canonical = location_section_label(label).unwrap_or("Available Locations");
        return Some((canonical, rest));
    }
    None
}

fn parse_single(raw: &str) -> ParsedLocation {
    let upper = raw.to_uppercase();
    let is_remote = upper.contains("REMOTE")
        || upper.contains("TELECOMMUTE")
        || upper.contains("ANYWHERE")
        || upper.contains("WORK FROM HOME")
        || upper.contains("VIRTUAL");

    let mut city: Option<String> = None;
    let mut region_code: Option<String> = None;
    let mut country_code: Option<String> = None;

    // Check city mapping
    for entry in TOP_CITIES {
        if upper.contains(entry.name) {
            city = Some(entry.city.to_string());
            region_code = Some(entry.region_code.to_string());
            country_code = Some(entry.country_code.to_string());
            break;
        }
    }

    // Check US state
    if region_code.is_none() {
        // Look for state abbreviation e.g. ", CA", " CA ", " CA," or full name
        for state in US_STATES {
            let abbr_pattern = format!(", {}", state.abbr);
            let abbr_pattern_end = format!(" {}", state.abbr);
            let is_match = upper.contains(state.name)
                || upper.ends_with(&abbr_pattern)
                || upper.ends_with(&abbr_pattern_end)
                || upper.contains(&format!("{}, ", state.abbr));

            if is_match {
                // Ambiguity check: "Georgia" vs "Tbilisi, Georgia"
                if state.abbr == "GA" && upper.contains("TBILISI") {
                    country_code = Some("GE".to_string());
                    region_code = None;
                    break;
                }
                region_code = Some(state.region_code.to_string());
                country_code = Some(state.country_code.to_string());
                break;
            }
        }
    }

    // Check CA provinces
    if region_code.is_none() {
        for prov in CA_PROVINCES {
            let abbr_pattern = format!(", {}", prov.abbr);
            let is_match = upper.contains(prov.name) || upper.ends_with(&abbr_pattern);
            if is_match {
                region_code = Some(prov.region_code.to_string());
                country_code = Some(prov.country_code.to_string());
                break;
            }
        }
    }

    // Check Country
    if country_code.is_none() {
        let normalized_country = upper.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if matches!(
            normalized_country,
            "US" | "USA" | "UNITED STATES" | "UNITED STATES OF AMERICA"
        ) || upper.contains("UNITED STATES")
            || upper.contains("USA")
            || upper.contains("U.S.")
            || upper.ends_with(" US")
        {
            country_code = Some("US".to_string());
        } else if upper.contains("CANADA") || upper.ends_with(" CA") {
            country_code = Some("CA".to_string());
        } else if upper.contains("UNITED KINGDOM") || upper.contains("UK") || upper.ends_with(" GB")
        {
            country_code = Some("GB".to_string());
        } else if upper.contains("GERMANY") || upper.contains("DEUTSCHLAND") {
            country_code = Some("DE".to_string());
        } else if upper.contains("IRELAND") {
            country_code = Some("IE".to_string());
        } else if upper.contains("NETHERLANDS") {
            country_code = Some("NL".to_string());
        } else if is_remote {
            country_code = None; // Global remote
        }
    }

    ParsedLocation {
        raw: raw.to_string(),
        city,
        region_code,
        country_code,
        is_remote,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_cities() {
        let loc = parse("San Francisco, CA").remove(0);
        assert_eq!(loc.city, Some("San Francisco".to_string()));
        assert_eq!(loc.region_code, Some("US-CA".to_string()));
        assert_eq!(loc.country_code, Some("US".to_string()));
        assert!(!loc.is_remote);

        let nyc = parse("NYC").remove(0);
        assert_eq!(nyc.city, Some("New York".to_string()));
        assert_eq!(nyc.region_code, Some("US-NY".to_string()));
    }

    #[test]
    fn test_location_remote() {
        let r1 = parse("Remote - US").remove(0);
        assert!(r1.is_remote);
        assert_eq!(r1.country_code, Some("US".to_string()));

        let r2 = parse("Remote (Canada)").remove(0);
        assert!(r2.is_remote);
        assert_eq!(r2.country_code, Some("CA".to_string()));

        let r3 = parse("Anywhere").remove(0);
        assert!(r3.is_remote);
    }

    #[test]
    fn test_compound_locations() {
        let locs = parse("San Francisco, CA; New York, NY; London, UK");
        assert_eq!(locs.len(), 3);
        assert_eq!(locs[0].region_code, Some("US-CA".to_string()));
        assert_eq!(locs[1].region_code, Some("US-NY".to_string()));
        assert_eq!(locs[2].country_code, Some("GB".to_string()));
    }

    #[test]
    fn test_georgia_ambiguity() {
        let us_ga = parse("Atlanta, Georgia").remove(0);
        assert_eq!(us_ga.region_code, Some("US-GA".to_string()));

        let country_ge = parse("Tbilisi, Georgia").remove(0);
        assert_eq!(country_ge.country_code, Some("GE".to_string()));
    }

    #[test]
    fn labeled_description_section_provides_location_evidence_and_provenance() {
        let description = super::super::normalize_description(
            "<h3>Available Locations</h3><ul><li>US</li><li>Canada</li><li>US, Canada</li></ul><h3>About Us</h3><p>We serve the United States.</p>",
            4096,
        );
        let extracted = from_description_sections(&description);
        let locations = extracted.locations;
        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].country_code.as_deref(), Some("US"));
        assert_eq!(locations[0].raw, "Available Locations: US");
        assert_eq!(locations[1].country_code.as_deref(), Some("CA"));
        assert!(locations.iter().all(|location| !location.is_remote));
    }

    #[test]
    fn general_company_prose_does_not_become_eligible_geography() {
        let description = super::super::normalize_description(
            "<h3>About the company</h3><p>We operate across the United States.</p>",
            4096,
        );
        assert!(from_description_sections(&description).locations.is_empty());
    }

    #[test]
    fn inline_labeled_location_is_supported_and_unknown_values_are_ignored() {
        let description = super::super::normalize_description(
            "<p><strong>Work Location:</strong> United States</p><p>Location: Every timezone in the galaxy</p>",
            4096,
        );
        let locations = from_description_sections(&description).locations;
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].country_code.as_deref(), Some("US"));
        assert_eq!(locations[0].raw, "Work Location: United States");
    }
}
