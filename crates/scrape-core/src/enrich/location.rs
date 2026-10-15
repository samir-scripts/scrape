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

impl ParsedLocation {
    /// A fully qualified subdivision is country evidence even in a partial legacy row.
    pub fn effective_country(&self) -> Option<&str> {
        self.country_code
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .or_else(|| {
                self.region_code
                    .as_deref()?
                    .split_once('-')
                    .map(|(prefix, _)| prefix)
            })
    }
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

/// Preserve the meaning of explicitly named address fields before free-text inference.
pub fn from_address(city: &str, region: &str, country: &str) -> Option<ParsedLocation> {
    use crate::geography::{country_code, normalize, REGIONS};
    let country_code = country_code(country)?;
    let region_code = if region.trim().is_empty() {
        None
    } else {
        REGIONS
            .iter()
            .find(|entry| {
                entry.country_code == country_code
                    && (entry.code.eq_ignore_ascii_case(region)
                        || normalize(&entry.name) == normalize(region))
            })
            .map(|entry| format!("{}-{}", entry.country_code, entry.code))
            .or_else(|| {
                if let Some((prefix, suffix)) = region.split_once('-') {
                    (prefix.eq_ignore_ascii_case(&country_code)
                        && !suffix.is_empty()
                        && suffix.len() <= 3
                        && suffix.chars().all(|ch| ch.is_ascii_alphanumeric()))
                    .then(|| region.to_ascii_uppercase())
                } else {
                    (!region.is_empty()
                        && region.len() <= 3
                        && region.chars().all(|ch| ch.is_ascii_alphanumeric()))
                    .then(|| format!("{country_code}-{}", region.to_ascii_uppercase()))
                }
            })
    };
    Some(ParsedLocation {
        raw: [city, region, country]
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        city: (!city.trim().is_empty()).then(|| city.to_string()),
        region_code,
        country_code: Some(country_code),
        is_remote: false,
    })
}

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

    // Keep country/subdivision names containing "and" intact.
    static SEPARATORS: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\s+or\s+|[;|/]").expect("valid location separators")
    });
    let parts: Vec<&str> = SEPARATORS
        .split(trimmed)
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
    use crate::geography::{contains_phrase, country_code as country, normalize, REGIONS};
    // Exclusions are not positive eligibility evidence. Keep the complete raw text.
    let upper = raw.to_ascii_uppercase();
    let evidence = [" EXCEPT ", " EXCLUDING ", " NOT ELIGIBLE "]
        .iter()
        .filter_map(|marker| upper.find(marker))
        .min()
        .map(|end| &raw[..end])
        .unwrap_or(raw);
    let normalized = normalize(evidence);
    let is_remote = [
        "REMOTE",
        "TELECOMMUTE",
        "ANYWHERE",
        "WORK FROM HOME",
        "VIRTUAL",
        "WORLDWIDE",
    ]
    .iter()
    .any(|word| contains_phrase(evidence, word));
    let city = TOP_CITIES
        .iter()
        .find(|city| contains_phrase(evidence, city.name));
    let parts: Vec<_> = evidence
        .split([',', '(', ')', ':', '[', ']'])
        .map(normalize)
        .filter(|p| !p.is_empty())
        .collect();
    let policy_stripped = |part: &str| {
        let mut value = part.to_string();
        for prefix in [
            "REMOTE ",
            "HYBRID ",
            "ON SITE ",
            "ONSITE ",
            "IN OFFICE ",
            "IN PERSON ",
            "BASED IN ",
        ] {
            if let Some(rest) = value.strip_prefix(prefix) {
                value = rest.to_string();
            }
        }
        value
            .strip_suffix(" ONLY")
            .unwrap_or(&value)
            .trim()
            .to_string()
    };
    let ambiguous_code = |part: &str| REGIONS.iter().any(|r| r.code == part);
    let explicit_country = parts
        .iter()
        .find_map(|part| {
            let candidate = policy_stripped(part);
            if candidate == "GEORGIA" {
                return if contains_phrase(evidence, "TBILISI") {
                    Some("GE".into())
                } else {
                    None
                };
            }
            if candidate.len() == 2 && ambiguous_code(&candidate) && normalized != candidate {
                return city
                    .filter(|c| c.country_code == candidate)
                    .map(|c| c.country_code.to_string());
            }
            country(&candidate)
        })
        .or_else(|| {
            let last = evidence.split_whitespace().next_back()?;
            (last.len() <= 3 && last == last.to_uppercase() && !ambiguous_code(last))
                .then(|| country(last))
                .flatten()
        });
    let region = REGIONS
        .iter()
        .filter_map(|region| {
            if explicit_country
                .as_ref()
                .is_some_and(|code| code != &region.country_code)
            {
                return None;
            }
            let canonical = format!("{}-{}", region.country_code, region.code);
            let name = normalize(&region.name);
            let full_name = parts.iter().any(|part| policy_stripped(part) == name)
                || normalized.ends_with(&format!(" {name}"));
            let iso = contains_phrase(evidence, &canonical);
            let abbreviation = parts.iter().any(|part| part == &region.code)
                || normalized.ends_with(&format!(" {}", region.code));
            let ambiguous_name = name == "GEORGIA" && explicit_country.is_none() && city.is_none();
            let abbreviation_safe = explicit_country.as_ref() == Some(&region.country_code)
                || city.is_some_and(|c| c.country_code == region.country_code)
                || (country(&region.code).is_none()
                    && REGIONS.iter().filter(|r| r.code == region.code).count() == 1);
            let priority = if iso {
                0
            } else if abbreviation && abbreviation_safe {
                1
            } else if full_name && !ambiguous_name {
                2
            } else {
                return None;
            };
            Some((priority, region))
        })
        .min_by_key(|(priority, _)| *priority)
        .map(|(_, region)| region);
    let country_code = explicit_country
        .or_else(|| region.map(|r| r.country_code.clone()))
        .or_else(|| city.map(|c| c.country_code.to_string()));
    let region_code = region
        .map(|r| format!("{}-{}", r.country_code, r.code))
        .or_else(|| {
            city.filter(|c| Some(c.country_code) == country_code.as_deref())
                .map(|c| c.region_code.to_string())
        });
    ParsedLocation {
        raw: raw.to_string(),
        city: city
            .filter(|c| Some(c.country_code) == country_code.as_deref())
            .map(|c| c.city.to_string()),
        region_code,
        country_code,
        is_remote,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn country_region_aliases_are_bounded_and_do_not_guess_ambiguous_codes() {
        for (raw, country, region) in [
            ("remote - us", Some("US"), None),
            ("Remote (U.S.A.)", Some("US"), None),
            ("Remote - US-CA", Some("US"), Some("US-CA")),
            ("London, Ontario, Canada", Some("CA"), Some("CA-ON")),
            ("Québec, Canada", Some("CA"), Some("CA-QC")),
            ("New Mexico", Some("US"), Some("US-NM")),
            ("Indiana", Some("US"), Some("US-IN")),
            ("Washington, DC, US", Some("US"), Some("US-DC")),
            ("Ukraine", Some("UA"), None),
            ("Remote (CA)", None, None),
            ("Georgia", None, None),
            ("Sukkur", None, None),
            ("Worldwide except US", None, None),
        ] {
            let location = parse(raw).remove(0);
            assert_eq!(location.country_code.as_deref(), country, "{raw}");
            assert_eq!(location.region_code.as_deref(), region, "{raw}");
        }
        let remote = parse("Remote - US; Toronto, Canada");
        assert_eq!(remote.len(), 2);
        assert_eq!(remote[0].country_code.as_deref(), Some("US"));
        assert_eq!(remote[1].country_code.as_deref(), Some("CA"));
    }

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
