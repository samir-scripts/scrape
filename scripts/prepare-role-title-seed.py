#!/usr/bin/env python3
"""Normalize pinned O*NET 31.0 CSV files into a role-title TSV seed."""

from __future__ import annotations

import argparse
import csv
import io
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "crates/scrape-core/data/occupations/seed_role_titles.tsv"
SOURCES = (
    ("Title", 1),
    ("Reported Job Title", 2),
    ("Job Title", 3),
)


def _rows(data: bytes, title_column: str, tier: int) -> list[tuple[str, str, int]]:
    try:
        text = data.decode("utf-8-sig")
    except UnicodeDecodeError as exc:
        raise ValueError(f"Invalid UTF-8 in O*NET tier {tier} source") from exc
    reader = csv.DictReader(io.StringIO(text, newline=""))
    required = {"O*NET-SOC Code", title_column}
    if not reader.fieldnames or not required.issubset(reader.fieldnames):
        raise ValueError(f"O*NET tier {tier} source is missing columns: {', '.join(sorted(required))}")
    result = []
    for row in reader:
        line = reader.line_num
        if None in row or any(value is None for value in row.values()):
            raise ValueError(f"Malformed CSV row at tier {tier}, line {line}")
        raw_code = (row.get("O*NET-SOC Code") or "").strip()
        title = (row.get(title_column) or "").strip()
        if not title:
            raise ValueError(f"Empty title at tier {tier}, line {line}")
        if len(raw_code) != 10 or raw_code[2] != "-" or raw_code[7] != "." or not (
            raw_code[:2].isdigit() and raw_code[3:7].isdigit() and raw_code[8:10].isdigit()
        ):
            raise ValueError(f"Invalid O*NET-SOC code at tier {tier}, line {line}: {raw_code!r}")
        if "all other" in title.casefold():
            continue
        if any(char in title for char in "\t\r\n"):
            raise ValueError(f"Invalid control character in title at tier {tier}, line {line}")
        result.append((raw_code[:7], title, tier))
    return result


def build_seed(occupation_data: bytes, reported_titles: bytes, alternate_titles: bytes) -> bytes:
    records: dict[str, tuple[str, str, int]] = {}
    for data, (title_column, tier) in zip(
        (occupation_data, reported_titles, alternate_titles), SOURCES, strict=True
    ):
        for soc_code, title, priority in _rows(data, title_column, tier):
            key = title.casefold()
            prior = records.get(key)
            candidate = (soc_code, title, priority)
            if prior is None or (priority, soc_code, title) < (prior[2], prior[0], prior[1]):
                records[key] = candidate
    ordered = sorted(records.values(), key=lambda row: (row[2], row[1].casefold(), row[1]))
    output = io.StringIO(newline="")
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow(("soc_code", "title", "priority_tier"))
    writer.writerows(ordered)
    return output.getvalue().encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--occupation-data", type=Path, required=True)
    parser.add_argument("--reported-titles", type=Path, required=True)
    parser.add_argument("--alternate-titles", type=Path, required=True,
                        help="O*NET 31.0 job_titles.csv (formerly Alternate Titles)")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    parser.add_argument("--check", action="store_true", help="fail if the normalized seed differs")
    args = parser.parse_args()
    try:
        inputs = []
        for path, (title_column, tier) in zip(
            (args.occupation_data, args.reported_titles, args.alternate_titles), SOURCES, strict=True
        ):
            data = path.read_bytes()
            try:
                _rows(data, title_column, tier)
            except ValueError as exc:
                raise ValueError(f"{path}: {exc}") from exc
            inputs.append(data)
        expected = build_seed(*inputs)
        if args.check:
            if not args.output.exists() or args.output.read_bytes() != expected:
                print(f"{args.output} is missing or stale", file=sys.stderr)
                return 1
            return 0
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(expected)
        print(f"Wrote {args.output}: {len(expected.splitlines()) - 1} unique titles")
    except (OSError, ValueError) as exc:
        print(f"role-title seed preparation failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
