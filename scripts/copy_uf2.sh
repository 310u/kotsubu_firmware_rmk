#!/usr/bin/env bash
set -euo pipefail

UF2_FILE="${1:-kotsubu-rmk.uf2}"

if [[ ! -f "$UF2_FILE" ]]; then
  echo "UF2 file not found: $UF2_FILE" >&2
  exit 1
fi

search_roots=(
  "/run/media/${USER:-}"
  "/media/${USER:-}"
  "/mnt"
)

targets=()
for root in "${search_roots[@]}"; do
  [[ -d "$root" ]] || continue
  while IFS= read -r -d '' info_file; do
    targets+=("$(dirname "$info_file")")
  done < <(find "$root" -maxdepth 3 -type f -name "INFO_UF2.TXT" -print0 2>/dev/null)
done

if [[ "${#targets[@]}" -eq 0 ]]; then
  cat >&2 <<'EOF'
UF2 bootloader drive was not found.
Put the XIAO BLE into bootloader mode first, then re-run this command.
The mounted drive should contain INFO_UF2.TXT.
EOF
  exit 1
fi

if [[ "${#targets[@]}" -gt 1 ]]; then
  echo "Multiple UF2 drives were found:" >&2
  printf '  %s\n' "${targets[@]}" >&2
  echo "Please keep only one UF2 bootloader drive mounted and retry." >&2
  exit 1
fi

target_dir="${targets[0]}"
target_path="${target_dir}/$(basename "$UF2_FILE")"

echo "Copying ${UF2_FILE} -> ${target_path}"
cp "$UF2_FILE" "$target_path"
sync
echo "UF2 copy finished."
