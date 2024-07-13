#!/bin/bash
set -ex

commands=(orgs-then-issues orgs-with-issues update-issues)
orgs=(jwodder wheelodex)

for cmd in "${commands[@]}"
do cargo build -r -p "$cmd"
done

mkdir -p outputs
for cmd in "${commands[@]}"
do cargo run -r -p "$cmd" -- -o "outputs/$cmd.json" "${orgs[@]}"
done
