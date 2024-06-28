#!/bin/sh

cd "$(dirname "$(readlink -f "$0")")" || exit 1

rm -Rf ../src/generated/*

flatc -o ../src/generated -r --gen-object-api --bfbs-comments --rust-serialize --rust-module-root-file \
    ../external/uc-hub-api/flatbuffers/types/uuid.fbs \
    ../external/uc-hub-api/flatbuffers/types/timestamp.fbs \
    ../external/uc-hub-api/flatbuffers/types/duration.fbs \
    ../external/uc-hub-api/flatbuffers/types/variable.fbs \
    ../external/uc-hub-api/flatbuffers/types/provider_definition.fbs \
    ../external/uc-hub-api/flatbuffers/types/provider.fbs \
    ../external/uc-hub-api/flatbuffers/types/state.fbs \
    ../external/uc-hub-api/flatbuffers/messages/provider_definition_changed_event.fbs \
    ../external/uc-hub-api/flatbuffers/messages/state_changed_event.fbs \
    ../external/uc-hub-api/flatbuffers/messages/read_providers_query_request.fbs \
    ../external/uc-hub-api/flatbuffers/messages/read_providers_query_response.fbs \
    ../external/uc-hub-api/flatbuffers/messages/read_provider_definition_query_request.fbs \
    ../external/uc-hub-api/flatbuffers/messages/read_provider_definition_query_response.fbs \
    ../external/uc-hub-api/flatbuffers/messages/providers_changed_event.fbs \
    ../external/uc-hub-api/flatbuffers/messages/read_variables_query_request.fbs \
    ../external/uc-hub-api/flatbuffers/messages/read_variables_query_response.fbs \
    ../external/uc-hub-api/flatbuffers/messages/variables_changed_event.fbs \
    ../external/uc-hub-api/flatbuffers/messages/write_variables_command.fbs 

# There is a bug in flatc where the generation of mod.rs does not include all generated files.
# https://github.com/google/flatbuffers/issues/8096
# We maintain our own complete mod.rs which is copied to the generated folder.
# !!! If you add more files to the flatc call above, you have to adjust ./mod.rs manually. !!!
cp files/mod.rs ../src/generated/mod.rs
