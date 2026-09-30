#!/bin/bash
set -ex

cd "$(dirname "$0")"

commands=(orgs-then-issues orgs-with-issues repos-and-issues)
orgs=(jwodder wheelodex)

for cmd in "${commands[@]}"
do cargo build -r -p "$cmd"
done

mkdir -p outputs
for cmd in "${commands[@]}"
do cargo run -r -p "$cmd" -- -o "outputs/$cmd.json" -R outputs/stats.json "${orgs[@]}"
done
