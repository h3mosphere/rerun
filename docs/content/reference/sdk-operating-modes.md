---
title: SDK Operating Modes
order: 7
---

There are many different ways of sending data to the Rerun Viewer depending on what you're trying to achieve and whether the viewer is running in the same process as your code, in another process, or even as a separate web application.

In the [official examples](/examples), these different modes of operation are exposed via a standardized set of flags that we'll cover below.
We will also demonstrate how you can achieve the same behavior in your own code.

## Operating Modes

The Rerun SDK provides 4 modes of operation: `spawn`, `connect`, `serve` & `save`.

All four of them are optional: when none of these modes are active, the client will simply buffer the logged data in memory, waiting for one of these modes to be enabled so that it can flush it.

### Spawn

This is the default behavior you get when running all of our Python & Rust examples, and is generally the most convenient when you're experimenting.

#### `Python`

Call [`rr.spawn`](https://ref.rerun.io/docs/python/latest/package/rerun/__init__/#rerun.spawn) once at the start of your program to start a Rerun Viewer in an external process and stream all the data to it via TCP. If an external viewer was already running, `spawn` will connect to that one instead of spawning a new one.

#### `Rust`

[`rerun::native_viewer::spawn`](https://docs.rs/rerun/latest/rerun/native_viewer/fn.spawn.html) spawns a new viewer on the main thread (for platform compatibility reasons) and continues executing user code on a new thread, streaming data between the two in real-time using an in-memory channel.

## Connect

Connects to a remote Rerun Viewer and streams all the data via TCP.

You will need to start a stand-alone viewer first by typing `rerun` in your terminal.

#### `Python`

[`rr.connect`](https://ref.rerun.io/docs/python/latest/package/rerun/__init__/#rerun.connect)

#### `Rust`

[`RecordingStream::connect`](https://docs.rs/rerun/latest/rerun/struct.RecordingStream.html#method.connect)

## Serve

This starts the web version of the Rerun Viewer in your browser, and streams data to it in real-time using WebSockets.

#### `Python`

Use [`rr.serve`](https://ref.rerun.io/docs/python/latest/package/rerun/__init__/#rerun.serve).

#### `Rust`

[`RecordingStream::serve`](https://docs.rs/rerun/latest/rerun/struct.RecordingStream.html#method.serve)

## Save

Streams all logging data into an `.rrd` file on disk, which can then be loaded into a stand-alone viewer.

To view the saved file, use `rerun path/to/file.rrd`.

⚠️  [RRD files don't yet handle versioning!](https://github.com/rerun-io/rerun/issues/873) ⚠️

#### `Python`

Use [`rr.save`](https://ref.rerun.io/docs/python/latest/package/rerun/__init__/#rerun.save).

#### `Rust`

Use [`RecordingStream::save`](https://docs.rs/rerun/latest/rerun/struct.RecordingStream.html#method.save).

## Adding the standard flags to your programs

We provide helpers for both Python & Rust to effortlessly add and properly handle all of these flags in your programs.

- For Python, checkout the [`script_helpers`](https://ref.rerun.io/docs/python/latest/package/rerun/script_helpers/) module.
- For Rust, checkout our [`clap`]() [integration](https://docs.rs/rerun/latest/rerun/clap/index.html).

Have a look at the [official examples](/examples) to see these helpers in action.
