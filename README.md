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

## References

### X11
* [X Selections, Cut Buffers, and Kill Rings](https://www.jwz.org/doc/x-cut-and-paste.html)
* [X11: How does “the” clipboard work?](https://www.uninformativ.de/blog/postings/2017-04-02/0/POySTING-en.html)
* [X11 Wait for and Get Clipboard Text](https://stackoverflow.com/questions/8755471/x11-wait-for-and-get-clipboard-text)

### WinAPI
* [Using the Clipboard](https://docs.microsoft.com/en-us/windows/desktop/dataxchg/using-the-clipboard)

