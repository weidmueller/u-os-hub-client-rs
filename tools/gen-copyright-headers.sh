# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

set -euo pipefail

copyright="Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>"

set -x

reuse annotate --license MIT --copyright "$copyright" --merge-copyrights --recursive --skip-unrecognised $files

exit 0

# Only check modified files, ignore deleted files and files with less than 4 lines changed
files=$(git diff --diff-filter=dcr -w --numstat origin/main..HEAD | awk '{if ($1+$2 > 3) print $3}')

reuse annotate --license MIT --copyright "$copyright" --merge-copyrights --recursive --skip-unrecognised $files

unrecognizedFiles=$(echo "$files" | grep '\.fbs$')

if [ -n "$unrecognizedFiles" ]; then
    reuse annotate --copyright="$copyright" --license=MIT --style=c --merge-copyrights $unrecognizedFiles
fi
