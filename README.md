# clojure-reader

[![Coverage Status](https://coveralls.io/repos/github/Grinkers/clojure-reader/badge.svg?branch=main)](https://coveralls.io/github/Grinkers/clojure-reader?branch=main)

## A crate to read Clojure.

### EDN [`(Extensible Data Notation)`](https://github.com/edn-format/edn)

## MSRV (minimal supported rust version)

   Stable minus 2 versions. Once stable (1.0.0), the plan is to indefinitely maintain the MSRV.

# Default Features

  The following features are enabled by default. To disable use this crate without default features.

## std

   When using no_std, this crate relies on `alloc`. You must supply your own `#[global_allocator]`.

## floats

   Pulls in the dependency `ordered-float` for Edn::Double. Without this feature, parsing floating-point numbers will result in an Err.

# Optional Features

   The following features are not enabled by default. To enable them all, use with
   ```toml
    clojure-reader  = { features = ["full"] }
   ```

## arbitrary-nums

   Enables parsing of arbitrary length/precision Ints and Decimals. Relies on `bigdecimal` and `num-bigint` crates.

# no_std

   See the [pico example](examples/pico) for a minimalistic example of using this crate with the raspberry pi pico (rp2040)
