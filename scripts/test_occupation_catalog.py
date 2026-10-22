"""Isolated validation tests for the offline occupation-catalog importer."""

import importlib.util
import io
import json
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("import-occupation-catalog.py")
SPEC = importlib.util.spec_from_file_location("occupation_importer", SCRIPT)
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


def workbook(rows, inline_values=()):
    """Build an XLSX worksheet using shared strings and selected inline strings."""
    strings = []
    indices = {}
    for row in rows:
        for value in row:
            if value not in indices:
                indices[value] = len(strings)
                strings.append(value)
    inline_values = set(inline_values)
    shared = '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">' + "".join(
        f"<si><t>{value}</t></si>" for value in strings
    ) + "</sst>"
    sheet_rows = []
    for rnum, row in enumerate(rows, 1):
        cells = "".join(
            f'<c r="{chr(65 + col)}{rnum}" t="inlineStr"><is><t>{value}</t></is></c>'
            if value in inline_values else f'<c r="{chr(65 + col)}{rnum}" t="s"><v>{indices[value]}</v></c>'
            for col, value in enumerate(row)
        )
        sheet_rows.append(f'<row r="{rnum}">{cells}</row>')
    sheet = '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>' + "".join(sheet_rows) + "</sheetData></worksheet>"
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr("xl/sharedStrings.xml", shared)
        archive.writestr("xl/worksheets/sheet1.xml", sheet)
    return buffer.getvalue()


def fixture_bls(details=None, groups=None):
    groups = groups or tuple(f"{major:02d}-0000" for major in range(11, 57, 2))
    rows = [[code, "Group " + code] for code in groups]
    if details is None:
        details = [f"{major:02d}-{number:04d}" for major in range(11, 57, 2) for number in range(1001, 1077, 2)][:867]
        details = ["15-1252" if code == "15-1001" else "15-1253" if code == "15-1003" else code for code in details]
    rows += [[code, "Occupation " + code] for code in details]
    return workbook(rows)


def valid_onet(rows=()):
    return json.dumps({"row": list(rows)}).encode()


class OccupationImporterTests(unittest.TestCase):
    def test_malformed_spreadsheet_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Malformed BLS workbook"):
            IMPORTER.build_catalog(b"not an xlsx", valid_onet())

    def test_malformed_onet_json_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Malformed O\\*NET JSON"):
            IMPORTER.build_catalog(fixture_bls(), b"{")

    def test_duplicate_detailed_codes_are_rejected(self):
        valid = [f"{major:02d}-{number:04d}" for major in range(11, 57, 2) for number in range(1001, 1077, 2)][:866]
        valid += ["15-1252", "15-1252"]
        with self.assertRaisesRegex(ValueError, "Duplicate BLS detailed code 15-1252"):
            IMPORTER.build_catalog(fixture_bls(details=valid), valid_onet())

    def test_missing_parent_group_is_rejected(self):
        details = [f"{major:02d}-{number:04d}" for major in range(11, 57, 2) for number in range(1001, 1077, 2)][:867]
        details[0] = "99-1252"
        with self.assertRaisesRegex(ValueError, "missing parent group 99-0000"):
            IMPORTER.build_catalog(fixture_bls(details=details), valid_onet())

    def test_alternate_label_retains_ambiguous_occupations_and_sources(self):
        records = [
            {"onetsoc_code": "15-1252.00", "job_title": " Data Engineer ", "sources": "s1, s2"},
            {"onetsoc_code": "15-1253.00", "job_title": "data engineer", "sources": " s2, s3 "},
        ]
        result = IMPORTER.build_catalog(fixture_bls(), valid_onet(records))
        matches = [title for title in result["titles"] if title["origin"] == "onet"]
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0], {
            "label": "Data Engineer",
            "soc_codes": ["15-1252", "15-1253"],
            "onetsoc_codes": ["15-1252.00", "15-1253.00"],
            "source_codes": ["s1", "s2", "s3"],
            "origin": "onet",
        })

    def test_inline_strings_and_missing_titles_are_handled(self):
        groups = [f"{major:02d}-0000" for major in range(11, 57, 2)]
        details = [f"{major:02d}-{number:04d}" for major in range(11, 57, 2) for number in range(1001, 1077, 2)][:867]
        details[0] = "15-1252"
        rows = [[code, "Group " + code] for code in groups]
        rows += [[code, "Software, comma-bearing title" if code == "15-1252" else "Occupation " + code] for code in details]
        source = workbook(rows, inline_values={"Software, comma-bearing title"})
        result = IMPORTER.build_catalog(source, valid_onet())
        self.assertIn("Software, comma-bearing title", [item["title"] for item in result["occupations"]])

    def test_exact_shape_version_origin_and_determinism(self):
        source = fixture_bls()
        onet = valid_onet()
        first = IMPORTER.build_catalog(source, onet)
        self.assertEqual(first, IMPORTER.build_catalog(source, onet))
        self.assertEqual(set(first), {"version", "groups", "occupations", "titles", "attribution"})
        self.assertEqual(first["version"], "bls-2018_onet-31.0")
        self.assertEqual({title["origin"] for title in first["titles"]}, {"bls"})


if __name__ == "__main__":
    unittest.main()
