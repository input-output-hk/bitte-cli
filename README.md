# Bitte CLI

This is a little tool that helps with deployments of Bitte clusters.

Bitte is a set of NixOS configurations that are provisioned using Terraform and
runs a cluster of Consul, Vault, and Nomad instances.

## Build this using nix

    nix build -o bitte

## Run this

    ./bitte --help

### Install cli tools outside of nix

To install the bitte tools, you will also need the following dependencies:

- [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- pkg-config
- openssl (linux only, darwin will use Security framework)
- zlib

To install:

```bash
  cargo install --path cli  # for the bitte cli
  cargo install --path iogo # for the iogo utility
```

## Setup the Bitte Environment

    export BITTE_FLAKE=git+ssh://git@github.com/input-output-hk/bitte
    export BITTE_CLUSTER=cvn-testnet
    export AWS_DEFAULT_REGION=eu-central-1
    export AWS_PROFILE=cvn-testnet

# Development

This program is written in [Rust](https://doc.rust-lang.org/stable/book) using
the [Tokio](https://tokio.rs/tokio/tutorial) runtime for asynchronous execution.

This is not a full guide by any means, but should serve as a good starting
point, and baseline check for understanding much of the code. A few of the
most critical concepts to understand are briefly outlined below.

## Rust

Rust's compiler is a bit different than any other mainstream language since it
validates and manages memory at compile time, rather than at runtime like a
garbase collected language, or not at all like C/C++.

Therefore, a basic understanding of Rust's
[ownership](https://doc.rust-lang.org/stable/book/ch04-00-understanding-ownership.html)
model will prove instrumental to working productively with the language.

The Rust community does an excellent job of keeping their materials up to date
and easy to follow, so be sure to use the resources linked in this section if
you need help.

## Futures

Another important detail to understand is that, unlike some other languages,
[futures](https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html)
in Rust are lazy by default. That means that a future will not begin execution
until `await` is called on it, e.g:

```rust
// execution of `foo` does not occur here
let foo = async { 3 * 40 };

// `bar` is not a future and so is evaluated immediately to 72
let bar = 8 * 9;

// `foo` is finally evaluated to 120 here.
// The program returns to it's previous context (the function that called it)
// until evaluation completes.
println!("{}", bar * foo.await);
```

A brief explanation of how to eval futures eagerly is given below.

## Tokio

Rust's standard library doesn't provide an asynchronous runtime on it's own, so
one must opt in to an external one to make use of its async/await tokens. Tokio
has become the _de facto_ async runtime of choice for many projects, since it
provides both an execution environment for futures, as well as a multi-threaded,
well optimized runtime.

### Eager Futures

As mentioned above, futures are lazy by default and do nothing until awaited.
With tokio, one can work around this when needed by [spawning](https://tokio.rs/tokio/tutorial/spawning)
a new Tokio thread to run the future in while continuing work on the current
thread.

Becuase spawning threads can increase control flow complexity, you should probably
avoid doing it by default, and wait until you make an optimization pass, finding
only the futures that could really benefit from it.
