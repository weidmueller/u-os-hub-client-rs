<!--
SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>

SPDX-License-Identifier: MIT
-->

<!-- A Markdown readme is needed because crates.io doesn't support asciidoc -->

# u-OS Data Hub - Rust Client 🦀

## Table of Contents

- [Introduction](#introduction)
- [Adding this crate to your Cargo project](#adding-this-crate-to-your-cargo-project)
- [Examples](#examples)
- [Questions / Issues / Contributions](#questions--issues--contributions)
- [License](#license)

## Introduction

Welcome to the Rust client for the u-OS Variable-NATS-API.
The API is part of the u-OS Data Hub.
A general explanation of the u-OS Data Hub can be found [here](https://support.weidmueller.com/online-documentation/latest/312416/en-GB/index.html#142143243146210187).

This library implements a Rust client to use the u-OS Variable-NATS-API as a provider or consumer.
The API specification can be found in the [u-os-hub-api](https://github.com/weidmueller/u-os-hub-api) repository.

## Adding this crate to your Cargo project

Add the following dependency to your Cargo.toml:

```toml
[dependencies]
u-os-hub-client = "0.2"
```

## Examples

The `examples` folder contains several examples demonstrating library usage.

See [CONTRIBUTING.adoc](CONTRIBUTING.adoc) if you want to build and run them.

## Questions / Issues / Contributions

If you have any questions or need help, please visit the [Weidmueller Support Center](https://support.weidmueller.com/support-center/).

If you would like to contribute to the project, please take a look at our [Contributing Guidelines](CONTRIBUTING.adoc).

## License

This project is licensed under the [MIT license](LICENSE).
