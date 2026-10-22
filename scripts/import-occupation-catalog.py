#!/usr/bin/env python3
"""Build the checked-in occupation catalog from the vendored BLS/O*NET sources."""

from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
import zipfile
import gzip
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = ROOT / "crates/scrape-core/data/occupations"
BLS_FILE = DATA_DIR / "bls-2018-structure.xlsx"
ONET_FILE = DATA_DIR / "onet-31.0-job-titles.json.gz"
OUTPUT_FILE = DATA_DIR / "catalog.json"
VERSION = "bls-2018_onet-31.0"
ATTRIBUTION = (
    "Includes information from the O*NET 31.0 Database by USDOL/ETA, used under CC BY 4.0. "
    "Scrape selected and reorganized these titles; USDOL/ETA has not approved, endorsed, or tested these modifications."
)
BLS_URL = "https://www.bls.gov/soc/2018/soc_structure_2018.xlsx"
ONET_URL = "https://www.onetcenter.org/dl_files/database/db_31_0_json/job_titles.json"
NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
CODE_RE = re.compile(r"\d{2}-\d{4}\Z")


def _cell_strings(archive: zipfile.ZipFile) -> list[list[str]]:
    """Read worksheet cell text using shared and inline strings."""
    try:
        shared_root = ET.fromstring(archive.read("xl/sharedStrings.xml"))
        shared = ["".join(t.text or "" for t in item.iter(f"{{{NS['m']}}}t")) for item in shared_root]
        sheet_root = ET.fromstring(archive.read("xl/worksheets/sheet1.xml"))
    except (KeyError, ET.ParseError) as exc:
        raise ValueError(f"Malformed BLS workbook: {exc}") from exc

    rows: list[list[str]] = []
    for row in sheet_root.findall(".//m:sheetData/m:row", NS):
        cells: list[str] = []
        for cell in row.findall("m:c", NS):
            kind = cell.get("t")
            if kind == "inlineStr":
                value = "".join(t.text or "" for t in cell.iter(f"{{{NS['m']}}}t"))
            else:
                value = cell.findtext("m:v", default="", namespaces=NS)
                if kind == "s" and value:
                    try:
                        value = shared[int(value)]
                    except (ValueError, IndexError) as exc:
                        raise ValueError("Malformed BLS workbook: invalid shared-string index") from exc
            cells.append(value.strip())
        rows.append(cells)
    return rows


def _parse_bls(data: bytes) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    try:
        import io
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            rows = _cell_strings(archive)
    except (zipfile.BadZipFile, OSError) as exc:
        raise ValueError(f"Malformed BLS workbook: {exc}") from exc

    groups: dict[str, str] = {}
    detailed: dict[str, str] = {}
    for row in rows:
        values = [value for value in row if value]
        for index, value in enumerate(values):
            if not CODE_RE.fullmatch(value):
                continue
            code = value
            title = next((candidate for candidate in values[index + 1:] if candidate), "")
            if not title:
                raise ValueError(f"Missing title for BLS code {code}")
            if code.endswith("0000"):
                if code in groups:
                    raise ValueError(f"Duplicate BLS group code {code}")
                groups[code] = title
            elif code[-1] != "0":
                if code in detailed:
                    raise ValueError(f"Duplicate BLS detailed code {code}")
                detailed[code] = title
            break

    if len(groups) != 23:
        raise ValueError(f"Expected 23 unique BLS major groups, found {len(groups)}")
    if len(detailed) != 867:
        raise ValueError(f"Expected 867 unique BLS detailed occupations, found {len(detailed)}")
    group_codes = set(groups)
    occupations = []
    for code, title in detailed.items():
        parent = code[:2] + "-0000"
        if parent not in group_codes:
            raise ValueError(f"BLS occupation {code} has missing parent group {parent}")
        occupations.append({"soc_code": code, "title": title, "group_code": parent})
    groups_result = [{"code": code, "name": name} for code, name in groups.items()]
    return groups_result, occupations


def _parse_onet(data: bytes, valid_soc: set[str]) -> list[dict[str, object]]:
    try:
        document = json.loads(data)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"Malformed O*NET JSON: {exc}") from exc
    if not isinstance(document, dict) or not isinstance(document.get("row"), list):
        raise ValueError("Malformed O*NET JSON: expected a row array")

    titles: dict[str, dict[str, object]] = {}
    for item in document["row"]:
        if not isinstance(item, dict):
            raise ValueError("Malformed O*NET JSON: row entries must be objects")
        code = item.get("onetsoc_code")
        if not isinstance(code, str) or not code.startswith("15-"):
            continue
        label = item.get("job_title")
        if not isinstance(label, str) or not label.strip():
            continue
        label = label.strip()
        soc = code[:7]
        if soc not in valid_soc:
            raise ValueError(f"O*NET code {code} maps to unmatched BLS occupation {soc}")
        source_codes = item.get("sources")
        if isinstance(source_codes, str):
            source_codes = [source.strip() for source in source_codes.split(",") if source.strip()]
        elif isinstance(source_codes, list) and all(isinstance(source, str) for source in source_codes):
            source_codes = [source.strip() for source in source_codes if source.strip()]
        else:
            raise ValueError(f"Malformed O*NET source codes for {code}")
        key = label.casefold()
        record = titles.setdefault(key, {"label": label, "soc_codes": set(), "onetsoc_codes": set(), "source_codes": set()})
        if (label.casefold(), label) < (str(record["label"]).casefold(), str(record["label"])):
            record["label"] = label
        record["soc_codes"].add(soc)
        record["onetsoc_codes"].add(code)
        record["source_codes"].update(source_codes)

    result = []
    for record in titles.values():
        result.append({
            "label": record["label"],
            "soc_codes": sorted(record["soc_codes"]),
            "onetsoc_codes": sorted(record["onetsoc_codes"]),
            "source_codes": sorted(record["source_codes"]),
            "origin": "onet",
        })
    return result


def build_catalog(bls_bytes: bytes, onet_bytes: bytes) -> dict:
    """Create the deterministic catalog from raw XLSX and O*NET JSON bytes."""
    groups, occupations = _parse_bls(bls_bytes)
    onet_titles = _parse_onet(onet_bytes, {item["soc_code"] for item in occupations})
    bls_titles = [
        {"label": item["title"], "soc_codes": [item["soc_code"]], "onetsoc_codes": [], "source_codes": [], "origin": "bls"}
        for item in occupations
    ]
    key = lambda item: (item["label"].casefold(), item["label"])
    return {
        "version": VERSION,
        "groups": sorted(groups, key=lambda item: item["code"]),
        "occupations": sorted(occupations, key=lambda item: item["soc_code"]),
        "titles": sorted(bls_titles + onet_titles, key=key),
        "attribution": ATTRIBUTION,
    }


def _encode(catalog: dict) -> bytes:
    return (json.dumps(catalog, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if catalog.json is not current")
    args = parser.parse_args()
    try:
        onet_json = gzip.decompress(ONET_FILE.read_bytes())
        expected = _encode(build_catalog(BLS_FILE.read_bytes(), onet_json))
        if args.check:
            try:
                actual = OUTPUT_FILE.read_bytes()
            except FileNotFoundError:
                actual = None
            if actual != expected:
                print(f"{OUTPUT_FILE.relative_to(ROOT)} is missing, malformed, or stale", file=sys.stderr)
                return 1
            return 0
        OUTPUT_FILE.write_bytes(expected)
    except (OSError, ValueError) as exc:
        print(f"occupation catalog import failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
