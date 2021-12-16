# Drogue Device template

This is a [cargo-generate](https://github.com/cargo-generate/cargo-generate) template for Drogue Device using the BBC:micro bit v2.

## Prerequisites

* [rustup](https://rustup.rs/)
* [cargo-generate](https://github.com/cargo-generate/cargo-generate) - `cargo install cargo-generate`
* [probe-run](https://github.com/knurling-rs/probe-run) - `cargo install probe-run`


## Generating the project

Run the following command and enter the required values when prompted:

```
cargo generate --git https://github.com/drogue-iot/device-template
```

Once generated, go to the newly created project directory.

## Running the example

Make sure your BBC micro:bit is connected and found by probe-run:

```
probe-run --list-probes
```

Then, run the application:

```
cargo run --release
```

Once running, press the 'A' button on the device to have the configured text scroll across the LED matrix.
