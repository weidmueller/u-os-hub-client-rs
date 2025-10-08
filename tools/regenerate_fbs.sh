#!/bin/bash

# SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
#
# SPDX-License-Identifier: MIT

set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")" || exit 1

# Optionally, allow overriding the destination path
dest_path="${DEST_PATH:-"../src/generated"}"
echo "Generating flatbuffers files to $dest_path"

rm -Rf $dest_path/*

api_path="../external/u-os-hub-api"

flatc -o $dest_path -r --gen-object-api --bfbs-comments --rust-serialize --rust-module-root-file \
    $api_path/flatbuffers/types/uuid.fbs \
    $api_path/flatbuffers/types/timestamp.fbs \
    $api_path/flatbuffers/types/duration.fbs \
    $api_path/flatbuffers/types/variable.fbs \
    $api_path/flatbuffers/types/provider_definition.fbs \
    $api_path/flatbuffers/types/provider.fbs \
    $api_path/flatbuffers/types/state.fbs \
    $api_path/flatbuffers/messages/provider_definition_changed_event.fbs \
    $api_path/flatbuffers/messages/state_changed_event.fbs \
    $api_path/flatbuffers/messages/read_providers_query_request.fbs \
    $api_path/flatbuffers/messages/read_providers_query_response.fbs \
    $api_path/flatbuffers/messages/read_provider_definition_query_request.fbs \
    $api_path/flatbuffers/messages/read_provider_definition_query_response.fbs \
    $api_path/flatbuffers/messages/providers_changed_event.fbs \
    $api_path/flatbuffers/messages/read_variables_query_request.fbs \
    $api_path/flatbuffers/messages/read_variables_query_response.fbs \
    $api_path/flatbuffers/messages/variables_changed_event.fbs \
    $api_path/flatbuffers/messages/write_variables_command.fbs 

# There is a bug in flatc where the generation of mod.rs does not include all generated files.
# https://github.com/google/flatbuffers/issues/8096
# We maintain our own complete mod.rs which is copied to the generated folder.
# !!! If you add more files to the flatc call above, you have to adjust ./mod.rs manually. !!!
# TODO: Check the state of the issue
cp files/mod.rs $dest_path/mod.rs

# Regenerate copyright headers in generated files
# For some reason recursive fails with the temp folder, so we manually find all files
echo "Annotating copyright headers in generated files under $dest_path"
copyright="Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>"
generated_files=$(find $dest_path -type f)
reuse annotate --license MIT --copyright "$copyright" $generated_files
