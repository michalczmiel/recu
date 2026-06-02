#!/usr/bin/env python3
"""Convert `recu calendar --json` output into an iCalendar (.ics) file.

Each charge on each day becomes one all-day VEVENT. UIDs are stable per
charge id + date, so re-importing an updated export updates events in place
instead of creating duplicates.

Usage:
    recu calendar --json | python3 calendar_to_ics.py > recu-2026-06.ics
    recu calendar --json --next | python3 calendar_to_ics.py -o next.ics
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import sys
from dataclasses import dataclass

_HEADER: tuple[str, ...] = (
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//recu//calendar//EN",
    "CALSCALE:GREGORIAN",
    "METHOD:PUBLISH",
)


def escape(text: str) -> str:
    """Escape a value per RFC 5545 (backslash, semicolon, comma, newline)."""
    return (
        text.replace("\\", "\\\\")
        .replace(";", "\\;")
        .replace(",", "\\,")
        .replace("\n", "\\n")
    )


def fold(line: str) -> str:
    """Fold content lines to <=75 octets, continuation lines start with a space."""
    raw = line.encode("utf-8")
    if len(raw) <= 75:
        return line
    chunks, start = [], 0
    first_limit, cont_limit = 75, 74  # continuation lines reserve a leading space
    limit = first_limit
    while start < len(raw):
        end = min(start + limit, len(raw))
        # don't split a multi-byte char
        while end < len(raw) and (raw[end] & 0xC0) == 0x80:
            end -= 1
        chunks.append(raw[start:end].decode("utf-8"))
        start = end
        limit = cont_limit
    return "\r\n ".join(chunks)


@dataclass
class Event:
    """A single all-day charge, ready to serialize as a VEVENT."""

    uid: str
    date: str  # YYYYMMDD
    summary: str
    stamp: str  # DTSTAMP, UTC

    def to_lines(self) -> list[str]:
        return [
            "BEGIN:VEVENT",
            f"UID:{self.uid}",
            f"DTSTAMP:{self.stamp}",
            f"DTSTART;VALUE=DATE:{self.date}",
            fold(f"SUMMARY:{escape(self.summary)}"),
            "TRANSP:TRANSPARENT",
            "END:VEVENT",
        ]


@dataclass
class IcsCalendar:
    """A calendar of events, renderable as an iCalendar (.ics) document."""

    events: list[Event]
    month: str | None = None

    @classmethod
    def from_json(cls, cal) -> "IcsCalendar":
        """Parse `recu calendar --json` output into an IcsCalendar."""
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        currency = cal.get("currency") or ""
        events = []
        for day in cal.get("days", []):
            date = day["date"].replace("-", "")  # YYYY-MM-DD -> YYYYMMDD
            for charge in day.get("charges", []):
                price = f"{charge['amount']} {currency}".strip()
                events.append(
                    Event(
                        uid=f"recu-{charge['id']}-{date}@recu",
                        date=date,
                        summary=f"{charge['name']} — {price}",
                        stamp=stamp,
                    )
                )
        return cls(events=events, month=cal.get("month"))

    def to_lines(self) -> list[str]:
        lines = list(_HEADER)
        for event in self.events:
            lines += event.to_lines()
        lines.append("END:VCALENDAR")
        return lines

    def render(self) -> str:
        """Serialize to a CRLF-terminated .ics string."""
        return "\r\n".join(self.to_lines()) + "\r\n"


def main():
    ap = argparse.ArgumentParser(description="Convert recu calendar --json to .ics")
    ap.add_argument("-o", "--output", help="write to this file instead of stdout")
    args = ap.parse_args()

    calendar = IcsCalendar.from_json(json.load(sys.stdin))
    ics = calendar.render()

    if args.output:
        with open(args.output, "w", newline="") as f:
            f.write(ics)
        count = len(calendar.events)
        print(
            f"Wrote {count} event(s) for {calendar.month} to {args.output}",
            file=sys.stderr,
        )
    else:
        sys.stdout.write(ics)


if __name__ == "__main__":
    main()
