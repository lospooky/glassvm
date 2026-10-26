#!/usr/bin/env bash
set -euo pipefail

source_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ "${1:-}" == "--output-root" ]]; then
  if [[ "$#" -ne 2 ]]; then
    echo "usage: $0 [--output-root DIR]" >&2
    exit 2
  fi
  proof_root="$2"
  mkdir -p "$proof_root"
else
  proof_root="$(mktemp -d /tmp/glassvm-external-python.XXXXXX)"
fi

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
    local project_env="$project_dir/.venv"
    UV_PROJECT_ENVIRONMENT="$project_env" uv sync --project "$project_dir" --locked --python 3.12 --no-cache
    UV_PROJECT_ENVIRONMENT="$project_env" uv run --project "$project_dir" --no-sync python prepare_fixtures.py
    if [[ "${#providers[@]}" -gt 0 ]]; then
      uv pip install --no-cache --python "$project_env/bin/python" "${providers[@]}"
    fi
    UV_PROJECT_ENVIRONMENT="$project_env" uv run --project "$project_dir" --no-sync python run_consumer.py \
      --expect "${machines[@]}" \
      --output "output/$name"
  )
}

run_case base --
run_case chip8 glassvm-chip8==0.3.1 -- chip8
run_case pico8 glassvm-pico8==0.3.1 -- pico8
run_case tic80 glassvm-tic80==0.3.1 -- tic80
run_case all glassvm-chip8==0.3.1 glassvm-pico8==0.3.1 \
  glassvm-tic80==0.3.1 -- chip8 pico8 tic80

printf 'External Python proof outputs: %s\n' "$proof_root"
