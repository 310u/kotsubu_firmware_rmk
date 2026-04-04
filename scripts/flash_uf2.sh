#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "${script_dir}/build_uf2.sh"
bash "${script_dir}/copy_uf2.sh" "kotsubu-rmk.uf2"
