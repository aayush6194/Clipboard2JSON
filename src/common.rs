use failure::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Defines common traits for the clipboard so that it's easier to abstract over
/// the underlying libraries.
pub trait ClipboardFunctions {
    /// Gets a list of all the clipboard format targets along with their name
    fn get_targets(&self) -> Result<ClipboardTargets, Error>;
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

/// Represents the different clipboard format target available in WinAPI and X11.
/// Both allow to get the target identifier along with their name but somewhat
/// differ in their representation of it.
#[derive(Debug)]
pub enum ClipboardTargets {
    WINAPI(HashMap<String, u32>),
    X11(HashMap<String, u64>),
}

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

impl ClipboardData {
    /// Helper function for getting the content stored for testing purpose
    #[allow(dead_code)]
    fn get_content(&self) -> String {
        match self {
            ClipboardData::Html { content, .. } => content.to_string(),
            ClipboardData::UnicodeText { content, .. } => content.to_string(),
        }
    }
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

/// Helper function for getting the timestamp in milliseconds when the
/// `ClipboardData` enum is created.
fn get_created_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Oops went back in time")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Clipboard;
    use clipboard::{ClipboardContext, ClipboardProvider};

    // Cannot set contents in X11 Clipboard
    // https://github.com/quininer/x11-clipboard/issues/9
    #[test]
    #[cfg(windows)]
    fn test_get_clipboard_text() {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let clipboard = Clipboard::new().unwrap();
        let data = "This is a normal string";
        ctx.set_contents(data.to_string()).unwrap();
        let clipboard_data = clipboard.get_clipboard().unwrap().get_content();
        assert_eq!(data.to_string(), clipboard_data);
    }
}
