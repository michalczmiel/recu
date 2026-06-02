#!/usr/bin/env python3
"""Behaviour-driven tests for calendar_to_ics.py.

We exercise the public surface (`recu calendar --json` shape -> rendered .ics)
and assert on the parsed output, not on private helpers. Run with:

    python3 -m unittest discover tests
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from calendar_to_ics import IcsCalendar


def render(cal):
    """Render a calendar dict to an .ics string."""
    return IcsCalendar.from_json(cal).render()


def unfold(ics):
    """Undo RFC 5545 line folding, returning logical lines."""
    return ics.replace("\r\n ", "").split("\r\n")


def events(ics):
    """Split rendered .ics into a list of {key: value} VEVENT blocks."""
    blocks, current = [], None
    for line in unfold(ics):
        if line == "BEGIN:VEVENT":
            current = {}
        elif line == "END:VEVENT":
            blocks.append(current)
            current = None
        elif current is not None and ":" in line:
            key, _, value = line.partition(":")
            current[key.split(";")[0]] = value
    return blocks


SAMPLE = {
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


class TestCalendarToIcs(unittest.TestCase):
    def test_produces_one_event_per_charge(self):
        self.assertEqual(len(events(render(SAMPLE))), 3)

    def test_wraps_events_in_a_valid_vcalendar(self):
        lines = unfold(render(SAMPLE))
        self.assertEqual(lines[0], "BEGIN:VCALENDAR")
        self.assertIn("VERSION:2.0", lines)
        self.assertEqual(lines[-2], "END:VCALENDAR")  # last entry is trailing ""

    def test_events_are_all_day_on_the_charge_date(self):
        first = events(render(SAMPLE))[0]
        self.assertEqual(first["DTSTART"], "20260601")

    def test_summary_carries_name_amount_and_currency(self):
        first = events(render(SAMPLE))[0]
        self.assertEqual(first["SUMMARY"], "Netflix — 15.99 USD")

    def test_uid_is_stable_across_re_exports(self):
        # Same charge + date must yield the same UID so re-imports update in
        # place rather than duplicating — the whole reason the script exists.
        uid_a = events(render(SAMPLE))[0]["UID"]
        uid_b = events(render(SAMPLE))[0]["UID"]
        self.assertEqual(uid_a, uid_b)
        self.assertEqual(uid_a, "recu-1-20260601@recu")

    def test_uses_crlf_line_endings(self):
        ics = render(SAMPLE)
        self.assertIn("\r\n", ics)
        self.assertNotIn("\n\n", ics.replace("\r\n", "\n"))  # no bare LFs

    def test_escapes_special_chars_in_names(self):
        cal = {
            "days": [
                {
                    "date": "2026-06-01",
                    "charges": [
                        {"id": 9, "name": "Adobe; Photoshop, Inc", "amount": "20"},
                    ],
                }
            ]
        }
        summary = events(render(cal))[0]["SUMMARY"]
        self.assertIn("\\;", summary)
        self.assertIn("\\,", summary)

    def test_empty_calendar_renders_no_events_but_stays_valid(self):
        ics = render({"month": "2026-06", "currency": "USD", "days": []})
        self.assertEqual(events(ics), [])
        self.assertIn("BEGIN:VCALENDAR", ics)
        self.assertIn("END:VCALENDAR", ics)

    def test_missing_currency_omits_trailing_space(self):
        cal = {
            "days": [
                {
                    "date": "2026-06-01",
                    "charges": [
                        {"id": 1, "name": "Netflix", "amount": "15.99"},
                    ],
                }
            ]
        }
        self.assertEqual(events(render(cal))[0]["SUMMARY"], "Netflix — 15.99")


if __name__ == "__main__":
    unittest.main()
