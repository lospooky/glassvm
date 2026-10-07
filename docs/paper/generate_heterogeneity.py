#!/usr/bin/env python3
"""Generate deterministic Markdown and LaTeX tables from PAPER_HETEROGENEITY.json."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


PAPER_DIR = Path(__file__).resolve().parent
DATA_PATH = PAPER_DIR / "PAPER_HETEROGENEITY.json"
OUTPUTS = {
    PAPER_DIR / "PAPER_HETEROGENEITY.md": "markdown",
    PAPER_DIR / "PAPER_HETEROGENEITY.tex": "latex",
}
REPOSITORIES = {
    "glassvm": "https://github.com/lospooky/glassvm",
    "machines": "https://github.com/lospooky/glassvm-machines",
}


def source_url(source: dict[str, str], revisions: dict[str, str]) -> str:
    repository = source["repository"]
    revision_key = "glassvm" if repository == "glassvm" else "glassvm-machines"
    return f"{REPOSITORIES[repository]}/blob/{revisions[revision_key]}/{source['path']}"


def validate(data: dict) -> None:
    order = data["machine_order"]
    sources = data["sources"]
    if len(order) != len(set(order)) or set(order) != set(data["machine_labels"]):
        raise ValueError("machine_order must list every machine label exactly once")
    revisions = data["evaluated_revisions"]
    if set(revisions) != {"glassvm", "glassvm-machines"}:
        raise ValueError("evaluated_revisions must pin both repositories")
    for source_id, source in sources.items():
        if source["repository"] not in REPOSITORIES:
            raise ValueError(f"unknown repository in source {source_id}")
        if not source["path"] or source["path"].startswith("/") or ".." in Path(source["path"]).parts:
            raise ValueError(f"source {source_id} has a non-repository-relative path")
    dimensions: set[str] = set()
    for row in data["rows"]:
        if row["dimension"] in dimensions:
            raise ValueError(f"duplicate dimension: {row['dimension']}")
        dimensions.add(row["dimension"])
        if set(row) != {"dimension", *order}:
            raise ValueError(f"row {row['dimension']} must have exactly one cell per machine")
        for machine in order:
            cell = row[machine]
            if not cell["text"].strip() or not cell["sources"]:
                raise ValueError(f"row {row['dimension']} / {machine} needs text and sources")
            unknown = set(cell["sources"]) - set(sources)
            if unknown:
                raise ValueError(f"unknown source IDs in {row['dimension']} / {machine}: {sorted(unknown)}")
    unknown = set(data["shared_contract"]["sources"]) - set(sources)
    if unknown:
        raise ValueError(f"unknown shared-contract source IDs: {sorted(unknown)}")


def markdown_cell(cell: dict, source_numbers: dict[str, int], sources: dict, revisions: dict) -> str:
    citations = " ".join(
        f"[\[{source_numbers[source_id]}\]]({source_url(sources[source_id], revisions)})"
        for source_id in cell["sources"]
    )
    return f"{cell['text']} {citations}"


def render_markdown(data: dict, source_numbers: dict[str, int]) -> str:
    order = data["machine_order"]
    sources = data["sources"]
    revisions = data["evaluated_revisions"]
    lines = [
        f"# {data['title']}",
        "",
        f"Evaluated GlassVM source `{revisions['glassvm']}` and machine source `{revisions['glassvm-machines']}`.",
        "",
        data["scope_note"],
        "",
        "## Shared caller lifecycle",
        "",
        markdown_cell(data["shared_contract"], source_numbers, sources, revisions),
        "",
        "## Machine heterogeneity",
        "",
        "| Dimension | " + " | ".join(data["machine_labels"][machine] for machine in order) + " |",
        "| --- | " + " | ".join("---" for _ in order) + " |",
    ]
    for row in data["rows"]:
        cells = [markdown_cell(row[machine], source_numbers, sources, revisions) for machine in order]
        lines.append("| " + row["dimension"] + " | " + " | ".join(cells) + " |")
    lines.extend(["", "## Source index", ""])
    for source_id, number in source_numbers.items():
        source = sources[source_id]
        lines.append(f"{number}. [{source['label']}]({source_url(source, revisions)}) (`{source['repository']}:{source['path']}`)")
    lines.append("")
    return "\n".join(lines)


def latex_escape(value: str) -> str:
    replacements = {
        "\\": r"\textbackslash{}",
        "&": r"\&",
        "%": r"\%",
        "$": r"\$",
        "#": r"\#",
        "_": r"\_",
        "{": r"\{",
        "}": r"\}",
        "~": r"\textasciitilde{}",
        "^": r"\textasciicircum{}",
    }
    return "".join(replacements.get(char, char) for char in value)


def latex_cell(cell: dict, source_numbers: dict[str, int], sources: dict, revisions: dict) -> str:
    text = latex_escape(cell["text"])
    citations = "".join(
        rf"\textsuperscript{{\href{{{source_url(sources[source_id], revisions)}}}{{[{source_numbers[source_id]}]}}}}"
        for source_id in cell["sources"]
    )
    return f"{text}{citations}"


def render_latex(data: dict, source_numbers: dict[str, int]) -> str:
    order = data["machine_order"]
    sources = data["sources"]
    revisions = data["evaluated_revisions"]
    cols = "p{0.16\\textwidth} " + " ".join("p{0.245\\textwidth}" for _ in order)
    lines = [
        "% Generated by generate_heterogeneity.py; do not edit by hand.",
        "% Requires \\usepackage{longtable,hyperref}",
        rf"\paragraph{{Shared caller lifecycle.}} {latex_cell(data['shared_contract'], source_numbers, sources, revisions)}",
        "",
        rf"\begin{{longtable}}{{{cols}}}",
        "\\textbf{Dimension} & " + " & ".join(rf"\textbf{{{latex_escape(data['machine_labels'][machine])}}}" for machine in order) + " \\\\",
        r"\hline",
        r"\endfirsthead",
        "\\textbf{Dimension} & " + " & ".join(rf"\textbf{{{latex_escape(data['machine_labels'][machine])}}}" for machine in order) + " \\\\",
        r"\hline",
        r"\endhead",
    ]
    for row in data["rows"]:
        cells = [latex_escape(row["dimension"])] + [latex_cell(row[machine], source_numbers, sources, revisions) for machine in order]
        lines.append(" & ".join(cells) + " \\\\")
        lines.append(r"\hline")
    lines.extend([r"\end{longtable}", "", r"\paragraph{Scope.} " + latex_escape(data["scope_note"]), ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if generated outputs are stale")
    args = parser.parse_args()
    data = json.loads(DATA_PATH.read_text(encoding="utf-8"))
    validate(data)
    source_numbers = {source_id: number for number, source_id in enumerate(data["sources"], start=1)}
    rendered = {
        path: render_markdown(data, source_numbers) if kind == "markdown" else render_latex(data, source_numbers)
        for path, kind in OUTPUTS.items()
    }
    stale = []
    for path, contents in rendered.items():
        if args.check:
            if not path.exists() or path.read_text(encoding="utf-8") != contents:
                stale.append(path.name)
        else:
            path.write_text(contents, encoding="utf-8", newline="\n")
    if stale:
        print("Generated heterogeneity outputs are stale: " + ", ".join(stale), file=sys.stderr)
        return 1
    if args.check:
        print("Heterogeneity outputs match their source data.")
    else:
        print("Generated PAPER_HETEROGENEITY.md and PAPER_HETEROGENEITY.tex.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
