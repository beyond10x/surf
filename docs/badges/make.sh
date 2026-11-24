#!/bin/sh
# Writes the README's badges as plain SVG files into <repo>/docs/badges. No outside service.
# usage: make-badges.sh <repo-root>
set -eu
out="$1/docs/badges"
mkdir -p "$out"

# badge <file> <label> <value> <colour>
badge() {
  file="$1"; label="$2"; value="$3"; colour="$4"
  lw=$(( ${#label} * 65 / 10 + 12 ))
  vw=$(( ${#value} * 65 / 10 + 12 ))
  w=$(( lw + vw ))
  lx=$(( lw / 2 ))
  vx=$(( lw + vw / 2 ))
  {
    printf '<svg xmlns="http://www.w3.org/2000/svg" width="%s" height="20" role="img" aria-label="%s: %s">\n' "$w" "$label" "$value"
    printf '  <title>%s: %s</title>\n' "$label" "$value"
    printf '  <clipPath id="r"><rect width="%s" height="20" rx="3"/></clipPath>\n' "$w"
    printf '  <g clip-path="url(#r)">\n'
    printf '    <rect width="%s" height="20" fill="#4a4a4a"/>\n' "$lw"
    printf '    <rect x="%s" width="%s" height="20" fill="%s"/>\n' "$lw" "$vw" "$colour"
    printf '  </g>\n'
    printf '  <g fill="#fff" text-anchor="middle" font-family="Verdana,DejaVu Sans,sans-serif" font-size="11">\n'
    printf '    <text x="%s" y="14">%s</text>\n' "$lx" "$label"
    printf '    <text x="%s" y="14">%s</text>\n' "$vx" "$value"
    printf '  </g>\n'
    printf '</svg>\n'
  } > "$out/$file"
}

green="#2e7d32"
blue="#1565c0"
grey="#757575"

badge spec.svg "spec" "ESS 0.97.4, valid" "$blue"
badge conformance.svg "conformance" "203 of 203 scenarios" "$green"
badge tests.svg "tests" "2099 passed" "$green"
badge dependencies.svg "third-party crates shipped" "5: ring 0.17.14 and 4 it needs" "$blue"
badge unsafe.svg "unsafe code" "forbidden" "$green"
badge complete.svg "standard sections marked complete" "0" "$grey"
badge roadmap.svg "roadmap" "10 of 22 waves delivered" "$blue"
ls "$out"
