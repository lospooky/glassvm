#!/usr/bin/env bash
set -euo pipefail

source_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
proof_root="$(mktemp -d /tmp/glassvm-external-python.XXXXXX)"

run_case() {
  local name="$1"
  shift
  local -a providers=()
  while [[ "$#" -gt 0 && "$1" != "--" ]]; do
    providers+=("$1")
    shift
  done
  if [[ "$#" -gt 0 ]]; then
    shift
  fi
  local -a machines=("$@")
  local project_dir="$proof_root/$name/python"
  mkdir -p "$proof_root/$name"
  cp -R "$source_dir" "$project_dir"
  (
    cd "$project_dir"
    uv sync --locked --python 3.12
    uv run --no-sync python prepare_fixtures.py
    if [[ "${#providers[@]}" -gt 0 ]]; then
      uv pip install --python .venv/bin/python "${providers[@]}"
    fi
    uv run --no-sync python run_consumer.py \
      --expect "${machines[@]}" \
      --output "output/$name"
  )
}

run_case base --
run_case chip8 glassvm-chip8==0.2.0 -- chip8
run_case pico8 glassvm-pico8==0.2.0 -- pico8
run_case tic80 glassvm-tic80==0.2.0 -- tic80
run_case all glassvm-chip8==0.2.0 glassvm-pico8==0.2.0 \
  glassvm-tic80==0.2.0 -- chip8 pico8 tic80

printf 'External Python proof outputs: %s\n' "$proof_root"
