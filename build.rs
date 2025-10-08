// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

extern crate flatc_rust;

use std::path::Path;

const FLATBUFFER_SRC_BASE_PATH: &str = "external/u-os-hub-api/flatbuffers";

#[allow(clippy::expect_used, clippy::panic)]
fn main() {
    println!("cargo:rerun-if-changed={}", FLATBUFFER_SRC_BASE_PATH);

    let dest_path = Path::new("target/flatbuffers/");

    let msg_path = Path::new(FLATBUFFER_SRC_BASE_PATH).join("messages");
    let types_path = Path::new(FLATBUFFER_SRC_BASE_PATH).join("types");
    let fbs_source_paths = [msg_path, types_path];

    let mut fbs_files = vec![];

    for fbs_source_path in &fbs_source_paths {
        if !fbs_source_path.exists() {
            panic!("The path '{}' does not exist. Please clone the 'u-os-hub-api' repository into the 'external' directory.", fbs_source_path.display());
        }

        for entry in std::fs::read_dir(fbs_source_path).expect("Failed to read fbs sources") {
            let entry = entry.expect("Failed to get directory entry");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("fbs") {
                fbs_files.push(path);
            }
        }
    }

    flatc_rust::run(flatc_rust::Args {
        inputs: &fbs_files.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        out_dir: dest_path,
        includes: &[],
        binary: false,
        json: false,
        schema: false,
        lang: "rust",
        extra: &[
            "--gen-object-api",
            "--bfbs-comments",
            "--rust-serialize",
            "--rust-module-root-file",
        ],
    })
    .expect("Failed to generate flatbuffer rust files");

    // There is a bug in flatc where the generation of mod.rs does not include all generated files.
    // https://github.com/google/flatbuffers/issues/8096
    // We maintain our own complete mod.rs which is copied to the generated folder.
    // !!! If you add more files to the flatc call above, you have to adjust ./mod.rs manually. !!!
    // TODO: Check the state of the issue
    std::fs::copy("tools/files/mod.rs", dest_path.join("mod.rs"))
        .expect("Failed to copy fixed mod.rs file");
}
