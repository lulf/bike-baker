# Bike-baker game

This is a simple baking game for micro bit. The goal is to "bake" something based on user input,
currently the button 'A'. Some chaos element is provided using the accelerometer, so that if you
shake the device too much while hitting the button, progress will be slow. Once all LEDs on the
matrix are lit, you have won.

Controls:

* Button 'A': Bake/make progress
* Button 'B': Reset game

## Prerequisites

* [rustup](https://rustup.rs/)
* [cargo-generate](https://github.com/cargo-generate/cargo-generate) - `cargo install cargo-generate`
* [probe-run](https://github.com/knurling-rs/probe-run) - `cargo install probe-run`


## Running the game

Make sure your BBC micro:bit is connected and found by probe-run:

```
probe-run --list-probes
```

Then, run the application:

```
cargo run --release
```
