#!/usr/bin/env bash
# Adds (or replaces) a Typst Universe package in the list embedded in inkpdf.
#
#   scripts/add-package.sh <name> <version> [--dependency]
#
# - downloads the official archive into packages/vendor/;
# - computes its sha256 digest and reads its license from typst.toml;
# - writes the entry to packages/lock.toml (sorted by name, then version).
#
# Role `selected` (default): version made available to templates, one per name; an
# existing `selected` entry with the same name is replaced. Role `dependency`: version
# present only because another package imports it.
# See specs/002-typst-packages/contracts/lock-file.md.
set -euo pipefail

usage() { echo "usage: $0 <name> <version> [--dependency]" >&2; exit 2; }
[[ $# -ge 2 && $# -le 3 ]] || usage
name=$1 version=$2 role=selected
if [[ $# -eq 3 ]]; then [[ $3 == --dependency ]] || usage; role=dependency; fi
[[ $name =~ ^[a-z0-9][a-z0-9-]*$ ]] || { echo "invalid name: $name" >&2; exit 2; }
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "invalid version: $version" >&2; exit 2; }

root=$(cd "$(dirname "$0")/.." && pwd)
vendor=$root/packages/vendor
lock=$root/packages/lock.toml
mkdir -p "$vendor"

archive="$name-$version.tar.gz"
url="https://packages.typst.org/preview/$archive"
tmp=$(mktemp)
trap 'rm -f "$tmp" "$tmp.lock"' EXIT
curl -fsSL "$url" -o "$tmp" || { echo "download failed: $url" >&2; exit 1; }

if command -v sha256sum >/dev/null; then
  sha=$(sha256sum "$tmp" | cut -d' ' -f1)
else
  sha=$(shasum -a 256 "$tmp" | cut -d' ' -f1)
fi
license=$(tar -xzOf "$tmp" typst.toml | sed -n 's/^[[:space:]]*license[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' | head -n1)
[[ -n $license ]] || license="UNKNOWN"
mv "$tmp" "$vendor/$archive"

# Existing entries -> "name|version|sha256|license|role" lines.
entries=$(
  if [[ -f $lock ]]; then
    awk -F' = ' '
      /^\[\[package\]\]/ { if (n) print n "|" v "|" s "|" l "|" r; n=v=s=l=r="" }
      $1=="name" { n=$2 } $1=="version" { v=$2 } $1=="sha256" { s=$2 }
      $1=="license" { l=$2 } $1=="role" { r=$2 }
      END { if (n) print n "|" v "|" s "|" l "|" r }
    ' "$lock" | tr -d '"'
  fi
)

# Drops the entry with the same version and, in selected mode, the previous selected version.
kept="" removed=""
while IFS='|' read -r n v s l r; do
  [[ -z ${n:-} ]] && continue
  if [[ $n == "$name" && ( $v == "$version" || ( $role == selected && $r == selected ) ) ]]; then
    [[ $v != "$version" ]] && removed+="$n-$v.tar.gz"$'\n'
    continue
  fi
  kept+="$n|$v|$s|$l|$r"$'\n'
done <<<"$entries"
kept+="$name|$version|$sha|$license|$role"$'\n'

{
  echo "# Typst packages embedded in inkpdf. Edit via scripts/add-package.sh."
  echo "# Contract: specs/002-typst-packages/contracts/lock-file.md"
  printf '%s' "$kept" | grep -v '^$' | sort -t'|' -k1,1 -k2,2V | while IFS='|' read -r n v s l r; do
    printf '\n[[package]]\nnamespace = "preview"\nname = "%s"\nversion = "%s"\nsha256 = "%s"\nlicense = "%s"\nrole = "%s"\n' "$n" "$v" "$s" "$l" "$r"
  done
} >"$tmp.lock"
mv "$tmp.lock" "$lock"

# Deletes the replaced archive once no entry references it anymore.
while IFS= read -r old; do
  [[ -z $old ]] && continue
  base=${old%.tar.gz}; oname=${base%-*}; over=${base##*-}
  if ! grep -q "name = \"$oname\"" "$lock" || ! awk -v n="$oname" -v v="$over" -F' = ' '
      /^\[\[package\]\]/ { cn=cv="" } $1=="name" { gsub(/"/,"",$2); cn=$2 }
      $1=="version" { gsub(/"/,"",$2); cv=$2; if (cn==n && cv==v) f=1 } END { exit !f }' "$lock"; then
    rm -f "$vendor/$old"
    echo "removed $old"
  fi
done <<<"$removed"

echo "$name $version ($role, $license) -> packages/lock.toml"
