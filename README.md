# Clipboard2JSON 

[![Build Status](https://travis-ci.org/lawrencek0/Clipboard2JSON.svg?branch=master)](https://travis-ci.org/lawrencek0/Clipboard2JSON)

Clipboard2JSON is a tool that watches the system clipboard and writes the
contents to a JSON file when the clipboard selection changes. It abstracts
over the WinAPI and the X11 library to provide a common interface for tranforming
clipboard data to JSON.

It currently works only on Windows and Linux with X11 Server.

## Usage

Make sure you have [git](https://git-scm.com/) and [rustup](https://rustup.rs/)
installed.

```
git clone https://github.com/lawrencek0/Clipboard2JSON.git
cd Clipboard2JSON/
cargo install
cargo run
```

You can supply your own custom callback function for when the clipboard content
changes like in [src/utils.rs](https://github.com/lawrencek0/Clipboard2JSON/blob/master/src/utils.rs).
Your function needs to implement the [ClipboardSink](https://github.com/lawrencek0/Clipboard2JSON/blob/master/src/common.rs)
i.e. it needs to be able to take the `ClipboardData` enum and return a `Result<(), Error>`
type.
