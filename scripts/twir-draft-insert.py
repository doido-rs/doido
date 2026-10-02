#!/usr/bin/env python3
"""Add one link to a This Week in Rust draft, in place.

    twir-draft-insert.py <draft.md> <title> <url> [section]

The draft is ``draft/<date>-this-week-in-rust.md`` in rust-lang/this-week-in-rust;
the section is a ``### `` heading under "Updates from Rust Community" and defaults to
Observations/Thoughts.

Exit 0 when the line was added, 3 when the URL is already anywhere in the draft,
2 when the section is not in the file.
"""

import sys


def insert(text: str, title: str, url: str, section: str) -> str | None:
    """The draft with the link added, or None when the section is missing."""
    lines = text.split("\n")
    heading = f"### {section}"
    try:
        start = lines.index(heading)
    except ValueError:
        return None
    end = start + 1
    while end < len(lines) and not lines[end].startswith("#"):
        end += 1
    body = lines[start + 1 : end]
    while body and not body[0].strip():
        body.pop(0)
    while body and not body[-1].strip():
        body.pop()
    body.append(f"* [{title}]({url})")
    lines[start + 1 : end] = [""] + body + [""]
    return "\n".join(lines)


def main(argv: list[str]) -> int:
    if len(argv) < 4:
        print(__doc__, file=sys.stderr)
        return 1
    path, title, url = argv[1:4]
    section = argv[4] if len(argv) > 4 else "Observations/Thoughts"
    with open(path, encoding="utf-8") as f:
        text = f.read()
    if url in text:
        print(f"{url} is already in {path}")
        return 3
    out = insert(text, title, url, section)
    if out is None:
        print(f"no '### {section}' section in {path}", file=sys.stderr)
        return 2
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(out)
    print(f"added to {section}: {title}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
