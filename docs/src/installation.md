# Installation

There are several ways to install Comtrya on a system: **cargo**, **shell script**, **provided binaries**, or **package managers**. Comtrya can also be **built from source**.

## Cargo installation

If your system has the Rust programming language tools such as cargo installed, cargo can be used to fetch the source and build a binary on your machine, placing it in location accessible by your path environment variable. First, ensure that the Rust programming language tool is installed by running the following:

```shell
cargo --version
```

You should see the version of cargo installed on your system printed out. If you get an error or nothing, please get the Rust tooling from the [Rust Website](https://www.rust-lang.org/tools/install) if you want to build from source.

Once you have cargo and rustc on your system, you can fetch the sources and build with the following command:

```shell
cargo install comtrya
```

## Shell Script

A shell script is provided to fetch a pre-compiled comtrya binary and install it onto your system (if your operating system is supported by the script). To do this, in your terminal or command prompt, run the following line:

```shell
curl -fsSL https://get.comtrya.dev | sh
```

Or, optionally, you can get a specific version of comtrya using the following one-liner:

```shell
curl -fsSL https://get.comtrya.dev | VERSION=v0.9.2  sh
```

## Precompiled-binaries

Pre-compiled binaries are also included on our [GitHub repository](https://github.com/comtrya/comtrya) under our [releases](https://github.com/comtrya/comtrya/releases/).

## Package managers

Some package managers may provide comtrya. Check with yours. At the time of writing this, comtrya is available in the Arch User Repository (AUR), Homebrew and Ravenports. If you are interested in packaging comtrya for another ports or packaging system, feel free to reach out to the team by opening an issue on our GitHub for support.

## Building from source

Building from source should be a straight forward task for anyone familiar with the Rust toolchian. It is recommended that you read through the [cargo book](https://doc.rust-lang.org/cargo/) and get familiar with it. Once you are, building is a matter of simply cloning our repository and compiling it. However, it is important to note that you may need to ensure you have the development libraries for openssl installed on your system. Check with your operating system and package manager what these packages are as they can often vary in naming between different systems.
