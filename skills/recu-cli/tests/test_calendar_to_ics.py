#!/usr/bin/env python3
"""Worked examples for calendar_to_ics.py — `recu calendar --json` -> `.ics`.

These read as documentation: each test pipes a realistic JSON calendar through
the script exactly as a user would and pins the *entire* rendered .ics, so the
input/output contract is visible at a glance. Run with:

    python3 -m unittest discover tests

Note: needs Python 3.10+ (the script uses `X | None` type syntax).
"""

import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent.parent / "scripts" / "calendar_to_ics.py"


def run(calendar):
    """Pipe a calendar dict through the script, return its .ics output.

    DTSTAMP is `now()`, so we blank it to `DTSTAMP:STAMP` to keep examples stable.
    """
    # raw bytes, not text=True — universal-newline mode would hide the CRLFs.
    out = subprocess.run(
        [sys.executable, str(SCRIPT)],
        input=json.dumps(calendar).encode(),
        capture_output=True,
        check=True,
    ).stdout.decode()
    return re.sub(r"DTSTAMP:\d{8}T\d{6}Z", "DTSTAMP:STAMP", out)


def ics(text):
    """A readable LF-authored .ics literal, converted to the real CRLF form."""
    return text.replace("\n", "\r\n")


class TestCalendarToIcs(unittest.TestCase):
    def test_a_months_charges_become_all_day_events(self):
        calendar = {
            "month": "2026-06",
            "currency": "USD",
            "days": [
                {
                    "date": "2026-06-01",
                    "charges": [
                        {"id": 1, "name": "Netflix", "amount": "15.99"},
                        {"id": 2, "name": "Spotify", "amount": "9.99"},
                    ],
                },
                {
                    "date": "2026-06-15",
                    "charges": [
                        {"id": 3, "name": "Gym", "amount": "40.00"},
                    ],
                },
            ],
        }

        self.assertEqual(
            run(calendar),
            ics(
                "BEGIN:VCALENDAR\n"
                "VERSION:2.0\n"
                "PRODID:-//recu//calendar//EN\n"
                "CALSCALE:GREGORIAN\n"
                "METHOD:PUBLISH\n"
                "BEGIN:VEVENT\n"
                "UID:recu-1-20260601@recu\n"
                "DTSTAMP:STAMP\n"
                "DTSTART;VALUE=DATE:20260601\n"
                "SUMMARY:Netflix — 15.99 USD\n"
                "TRANSP:TRANSPARENT\n"
                "END:VEVENT\n"
                "BEGIN:VEVENT\n"
                "UID:recu-2-20260601@recu\n"
                "DTSTAMP:STAMP\n"
                "DTSTART;VALUE=DATE:20260601\n"
                "SUMMARY:Spotify — 9.99 USD\n"
                "TRANSP:TRANSPARENT\n"
                "END:VEVENT\n"
                "BEGIN:VEVENT\n"
                "UID:recu-3-20260615@recu\n"
                "DTSTAMP:STAMP\n"
                "DTSTART;VALUE=DATE:20260615\n"
                "SUMMARY:Gym — 40.00 USD\n"
                "TRANSP:TRANSPARENT\n"
                "END:VEVENT\n"
                "END:VCALENDAR\n"
            ),
        )

    def test_special_chars_are_escaped_and_currency_is_optional(self):
        calendar = {
            "days": [
                {
                    "date": "2026-06-01",
                    "charges": [
                        {"id": 9, "name": "Adobe; Photoshop, Inc", "amount": "20"},
                    ],
                }
            ]
        }

        self.assertEqual(
            run(calendar),
            ics(
                "BEGIN:VCALENDAR\n"
                "VERSION:2.0\n"
                "PRODID:-//recu//calendar//EN\n"
                "CALSCALE:GREGORIAN\n"
                "METHOD:PUBLISH\n"
                "BEGIN:VEVENT\n"
                "UID:recu-9-20260601@recu\n"
                "DTSTAMP:STAMP\n"
                "DTSTART;VALUE=DATE:20260601\n"
                "SUMMARY:Adobe\\; Photoshop\\, Inc — 20\n"
                "TRANSP:TRANSPARENT\n"
                "END:VEVENT\n"
                "END:VCALENDAR\n"
            ),
        )


if __name__ == "__main__":
    unittest.main()
