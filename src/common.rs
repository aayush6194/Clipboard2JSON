use failure::Error;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Defines common traits for the clipboard so that it's easier to abstract over
/// the underlying libraries.
// @TODO: Add more functions when workng on WINAPI
pub trait ClipboardFunctions: Sized {
    /// Creates a new `Clipboard` with a pointer to the hidden window
    // @TODO: Better error handling?
    fn new() -> Result<Self, Error>;
    /// Fetches the data stored in the clipboard as a text-based format
    fn get_clipboard(&self) -> Result<ClipboardData, Error>;
    /// Watches over the clipboard and passes the changed data to the callback
    fn watch_clipboard(&self, callback: &ClipboardSink);
}

/// Stores a function that takes the clipboard data and writes it to a source.
/// It is stored in a struct because it is easier to implement Clone this way which
/// plays nicely with the static variables in the WinAPI implementation of the
/// clipboard.
#[derive(Clone)]
pub struct ClipboardSink(pub fn(ClipboardData) -> Result<(), Error>);

/// Represents the textual data stored in clipboard as either HTML or UTF8.  
///
/// If the clipboard data can be converted to HTML, the owner also includes
/// the enclosing HTML tags around the content which can be used to format the
/// content differently. Also, the clipboard owner can convert types such as images
/// to an img tag with the URL for the image.Unlike the Win API, there does not
/// seem to be an easy way of getting the URL of the HTML document in X11.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ClipboardData {
    Html {
        content: String,
        owner: Option<String>,
        url: Option<String>,
        created_at: u64,
    },
    #[serde(rename = "text")]
    UnicodeText {
        content: String,
        owner: Option<String>,
        created_at: u64,
    },
}

impl From<(String, Option<String>, Option<String>)> for ClipboardData {
    fn from((content, owner, url): (String, Option<String>, Option<String>)) -> ClipboardData {
        ClipboardData::Html {
            content,
            owner,
            url,
            created_at: get_created_timestamp(),
        }
    }
}

impl From<(String, Option<String>)> for ClipboardData {
    fn from((content, owner): (String, Option<String>)) -> ClipboardData {
        ClipboardData::UnicodeText {
            content,
            owner,
            created_at: get_created_timestamp(),
        }
    }
}

impl ClipboardData {
    pub fn new<A>(args: A) -> Self
    where
        A: Into<Self>,
    {
        args.into()
    }
}

fn get_created_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Oops went back in time")
        .as_secs()
}
