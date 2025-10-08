#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

# This script sets the version of the repository inside all Cargo.toml files.

NEW_VERSION=$1

set -eu

# Copied from https://semver.org/#is-there-a-suggested-regular-expression-regex-to-check-a-semver-string
SEMVER_REGEX="^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"

if ! echo ${NEW_VERSION} | grep -Pq "${SEMVER_REGEX}"; then
    echo "Error: '$NEW_VERSION' is a invalid version format. Please use semantic versioning (e.g., 1.0.0)"
    exit 1
fi

REPOSITORY_ROOT="$(dirname "$(readlink -f "$0")")/.."

if [ -z "$NEW_VERSION" ]; then
    echo "Error: New version is missing"
    echo "Please pass the new version as the first argument e.g ./tools/set_version.sh 1.0.0"
    exit 1
fi

CRATES=$(find $REPOSITORY_ROOT -iname Cargo.toml -not -path "$REPOSITORY_ROOT/target/*")

echo "--> Setting version for all crates to $NEW_VERSION"
for cargo_file_path in $CRATES ; do
    echo "--> Setting version for $cargo_file_path"
    sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$cargo_file_path"
done

echo "--> Done, the new version '$NEW_VERSION' should be set in all crates and npm packages. Please verify the changes."
