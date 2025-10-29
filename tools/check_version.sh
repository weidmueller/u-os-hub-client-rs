#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

# This scripts compares the current version of the repository with the version set in Cargo.toml files.

set -euo pipefail

REPOSITORY_ROOT="$(dirname "$(readlink -f "$0")")/.."

CRATES=$(find $REPOSITORY_ROOT -iname Cargo.toml -not -path "$REPOSITORY_ROOT/target/*")

# Function to extract version from Cargo.toml
get_cargo_version() {
    grep -m1 "^version = " "$1" | sed 's/^version = "\(.*\)"/\1/'
}

# Initialize variables
reference_version=""
inconsistent=false

echo "Checking component versions..."

# Check all crates
for toml_file_path in $CRATES; do
    version=$(get_cargo_version "$toml_file_path")

    if [ -z "$reference_version" ]; then
        reference_version=$version
        echo "Reference version from $toml_file_path: $reference_version"
    fi

    if [ "$version" != "$reference_version" ]; then
        echo "❌ $toml_file_path has inconsistent version: $version (expected: $reference_version)"
        inconsistent=true
    else
        echo "✅ $toml_file_path: $version"
    fi
done

# Report final status
if [ "$inconsistent" = true ]; then
    echo "❌ Version inconsistency detected! Some components have different versions."
    exit 1
else
    echo "✅ All components have the same version: $reference_version"
fi

# Check if version matches the latest changelog entry
# make sure the reference version matches the latest entry in VERSION_HISTORY.md
changelog_file="$REPOSITORY_ROOT/CHANGELOG.md"

# Use grep with Perl-compatible regex (-P) to find lines starting with '### '
# followed by a semantic version pattern (X.Y.Z followed by optional pre-release tags).
# The -o option prints only the matching part of the line.
# The \K sequence tells grep to discard the part of the match before it ('### ').
# head -n 1 takes the first match, assuming it's the latest version.
changelog_latest_version=$(grep -oP '^### \K[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?' "$changelog_file" | head -n 1)

if [ "$reference_version" == "$changelog_latest_version" ]; then
    echo "✅ App version '$reference_version' matches latest entry in version changelog."
else
    echo "❌ App version '$reference_version' does NOT match latest entry in version changelog '$changelog_latest_version'"
    exit 1
fi


# Check if generated flatbuffers files are up to date
echo "👀 Checking if generated flatbuffers files are up to date..."

fbs_generation_dir="$REPOSITORY_ROOT/src/generated"
tmp_generated_dir="$REPOSITORY_ROOT/src/tmp_generated"

DEST_PATH="$tmp_generated_dir" $REPOSITORY_ROOT/tools/regenerate_fbs.sh

diff_rc=0
diff -rq "$tmp_generated_dir" "$fbs_generation_dir" || diff_rc=$?
rm -rf "$tmp_generated_dir"

if [ "$diff_rc" -eq 0 ]; then
    echo "✅ Generated flatbuffers files are up to date!"
elif [ "$diff_rc" -eq 1 ]; then
    echo "❌ Generated flatbuffers files are not up to date. Please run 'tools/regenerate_fbs.sh' to update them."
    exit 1
else
    echo "Diff encountered an error (code: $diff_rc)."
    exit 1
fi
