#!/bin/bash
set -ex

cd "$(dirname "$0")"

#commands=(orgs-then-issues orgs-with-issues repos-and-issues)
# As of 2026-09-25, orgs-with-issues fails due to a RESOURCE_LIMITS_EXCEEDED
# error; cf.
# <https://github.blog/changelog/2025-09-01-graphql-api-resource-limits/>.
commands=(orgs-then-issues repos-and-issues)
orgs=(jwodder wheelodex)

for cmd in "${commands[@]}"
do cargo build -r -p "$cmd"
done

mkdir -p outputs
for cmd in "${commands[@]}"
do cargo run -r -p "$cmd" -- -o "outputs/$cmd.json" -R outputs/stats.json "${orgs[@]}"
done
