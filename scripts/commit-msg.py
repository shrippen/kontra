#!/usr/bin/env python3
"""Commit message rules 1-6 of agent.md (W4), for the commit-msg hook and the CI.

    scripts/commit-msg.py FILE          check one message (the hook: .git/COMMIT_EDITMSG)
    scripts/commit-msg.py --range A..B  check every commit in A..B but merges (CI, pre-push)
    scripts/commit-msg.py --self-test   check the checker

Errors fail, warnings only print. Rule 7 (what and why) needs a reader, the review checks it.
Rule 5 (imperative) can only be guessed: subjects starting with a past or third-person form
of a common verb ("Added", "Fixes") fail.

    Subject line, at most 50 characters  <- rules 2-5
                                         <- rule 1
    Body wrapped at 72 characters, says  <- rule 6
    what and why.
"""
import re
import subprocess
import sys

SUBJECT_SOFT = 50
SUBJECT_HARD = 72
BODY_WIDTH = 72
# A German date ends in a period that is part of it ("Record the QA night of 27.10.").
DATE_END = re.compile(r"\d{1,2}\.\d{1,2}\.(\d{2,4})?$")
# Not imperative: "Added …", "Adds …", "Fixed …", "Fixing …".
NOT_IMPERATIVE = re.compile(r"^[A-Z][a-z]+(ed|es|ing)\b")
VERB_FORMS = re.compile(r"^(Add|Fix|Update|Remove|Change|Move|Rename|Use|Make|Show|Keep|Let|"
                        r"Drop|Clean|Refactor|Improve|Bump|Merge|Replace|Write|Test|Check|"
                        r"Implement|Create|Delete|Allow|Support|Handle|Split|Measure|Record)"
                        r"(s|ed|ing|es|d)\b")
# Long body lines that cannot be wrapped: links, paths, indented code or quotes.
UNWRAPPABLE = re.compile(r"^(\s{2,}|\t|>)|^\S+$|https?://")
COMMENT = "#"
SCISSORS = "# ------------------------ >8 ------------------------"


def lines_of(message):
    """The message as git stores it: comments and the diff below the scissors removed."""
    kept = []
    for line in message.splitlines():
        if line.startswith(SCISSORS):
            break
        if line.startswith(COMMENT):
            continue
        kept.append(line.rstrip())
    while kept and not kept[-1]:
        kept.pop()
    while kept and not kept[0]:
        kept.pop(0)
    return kept


def check(message):
    """(errors, warnings) of one message."""
    lines = lines_of(message)
    errors, warnings = [], []
    if not lines:
        return ["empty message"], warnings
    subject = lines[0]

    # Fixups and squashes are folded away before the merge.
    if subject.startswith(("fixup! ", "squash! ", "amend! ")):
        return errors, warnings

    if len(subject) > SUBJECT_HARD:
        errors.append(f"rule 2: subject has {len(subject)} characters, at most {SUBJECT_HARD}")
    elif len(subject) > SUBJECT_SOFT:
        warnings.append(f"rule 2: subject has {len(subject)} characters, aim for {SUBJECT_SOFT}")
    if not subject[0].isupper():
        errors.append("rule 3: subject must start with a capital letter")
    if subject.endswith(".") and not DATE_END.search(subject):
        errors.append("rule 4: subject must not end with a period")
    if VERB_FORMS.match(subject):
        errors.append(f"rule 5: imperative mood (\"{subject.split()[0]}\" -> "
                      f"\"Fix\", \"Add\", ...)")
    elif NOT_IMPERATIVE.match(subject):
        warnings.append(f"rule 5: is \"{subject.split()[0]}\" imperative?")

    if len(lines) > 1 and lines[1]:
        errors.append("rule 1: blank line between subject and body")
    for n, line in enumerate(lines[2:], start=3):
        if len(line) > BODY_WIDTH and not UNWRAPPABLE.search(line):
            errors.append(f"rule 6: line {n} has {len(line)} characters, wrap at {BODY_WIDTH}")
    return errors, warnings


def report(name, message):
    errors, warnings = check(message)
    for w in warnings:
        print(f"{name}: warning: {w}", file=sys.stderr)
    for e in errors:
        print(f"{name}: {e}", file=sys.stderr)
    return not errors


def check_range(span):
    """Every non-merge commit in `span` (git rev-list syntax)."""
    shas = subprocess.check_output(["git", "rev-list", "--no-merges", span], text=True).split()
    ok = True
    for sha in shas:
        message = subprocess.check_output(["git", "log", "-1", "--format=%B", sha], text=True)
        ok = report(sha[:9], message) and ok
    return ok


def self_test():
    good = "Count a note's last string verdict\n\nThe tracker told every verdict.\n"
    cases = [
        (good, 0, 0),
        ("Record the QA night of 27.10.", 0, 0),
        ("Fix it\n\n    a code line that is far longer than seventy-two characters, kept as is\n",
         0, 0),
        ("fixup! anything goes.", 0, 0),
        ("Add a subject that is longer than fifty characters but short", 0, 1),
        ("A" * 73, 1, 0),
        ("lowercase start", 1, 0),
        ("Ends with a period.", 1, 0),
        ("Added the feature", 1, 0),
        ("Fixes the bug", 1, 0),
        ("Subject\nno blank line", 1, 0),
        ("Subject\n\n" + "word " * 20, 1, 0),
        ("Shipped it", 0, 1),
        ("# only a comment\n", 1, 0),
        (good + SCISSORS + "\n" + "x " * 80 + "\n", 0, 0),
    ]
    failed = 0
    for message, n_errors, n_warnings in cases:
        errors, warnings = check(message)
        if (len(errors), len(warnings)) != (n_errors, n_warnings):
            failed += 1
            print(f"FAIL {message[:40]!r}: {errors} {warnings}", file=sys.stderr)
    print(f"{len(cases) - failed} of {len(cases)} passed")
    return failed == 0


def main(argv):
    if argv[1:] == ["--self-test"]:
        return 0 if self_test() else 1
    if len(argv) == 3 and argv[1] == "--range":
        return 0 if check_range(argv[2]) else 1
    if len(argv) == 2:
        with open(argv[1], encoding="utf-8") as f:
            return 0 if report("commit message", f.read()) else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
