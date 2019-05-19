use crate::common::{ClipboardFunctions, ClipboardSink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;
use std::os::raw::{c_char, c_int, c_long, c_uchar, c_ulong};
use std::time::{SystemTime, UNIX_EPOCH};
use x11::xlib::{
    AnyPropertyType, Atom, CurrentTime, Display, False, SelectionNotify, Window, XCloseDisplay,
    XConvertSelection, XCreateSimpleWindow, XDefaultRootWindow, XDeleteProperty, XDestroyWindow,
    XEvent, XFetchName, XGetAtomName, XGetSelectionOwner, XGetWindowProperty, XInternAtom,
    XNextEvent, XOpenDisplay, XSelectInput, XA_ATOM,
};

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
        owner: String,
        content: String,
        created_at: u64,
    },
    #[serde(rename = "text")]
    UnicodeText {
        owner: String,
        content: String,
        created_at: u64,
    },
}

/// Represents a windowless X11 Client and its connection to the X11 Server.
///
/// Please note that it does not currently handle large buffers.
pub struct Clipboard {
    /// Connection to the X11 Server
    display: *mut Display,
    /// Unmapped subwindow which is used for listening to events
    window: Window,
    /// Property on the window for reading the selection
    prop_id: Atom,
}

/// Functions from the XFixes extension that are used to notify the
/// window when the clipboard content chagnes. These functions are currently
/// not included in the X11 bindings.
extern "C" {
    fn XFixesSelectSelectionInput(_4: *mut Display, _3: Window, _2: Atom, _1: c_ulong);
    fn XFixesQueryExtension(_3: *mut Display, _2: *mut c_int, _1: *mut c_int) -> c_int;
}

impl Clipboard {
    /// Gets a hashmap of content type targets along with their atom identifier
    /// that the clipboard owner can convert the data to. The current implementation
    /// only handles HTML and text based formats i.e. text/html, UTF8_STRING, TEXT
    pub fn get_targets(&self) -> Option<HashMap<String, Atom>> {
        let mut event: XEvent = unsafe { mem::uninitialized() };
        let targets_id = unsafe {
            XInternAtom(
                self.display,
                CString::new("TARGETS").unwrap().as_ptr(),
                False,
            )
        };
        let clipboard_id = unsafe {
            XInternAtom(
                self.display,
                CString::new("CLIPBOARD").unwrap().as_ptr(),
                False,
            )
        };

        unsafe {
            // Listen to event when the selection is transferred
            XSelectInput(self.display, self.window, SelectionNotify.into());
            // Request the owner to send the targets it can convert the clipboard to
            XConvertSelection(
                self.display,
                clipboard_id,
                targets_id,
                self.prop_id,
                self.window,
                CurrentTime,
            );

            loop {
                XNextEvent(self.display, &mut event);

                if event.type_ == SelectionNotify || event.selection.selection == clipboard_id {
                    break;
                }
            }

            if event.selection.property != 0 {
                let mut return_type_id: Atom = mem::uninitialized();
                let mut return_format: c_int = 0;
                let mut returned_items: c_ulong = 0;
                let mut bytes_left: c_ulong = 0;
                let mut result: *mut c_uchar = mem::uninitialized();

                // Gets the size of targets to be transferred
                XGetWindowProperty(
                    self.display,
                    self.window,
                    self.prop_id,
                    0,
                    0,
                    False,
                    XA_ATOM,
                    &mut return_type_id,
                    &mut return_format,
                    &mut returned_items,
                    &mut bytes_left,
                    &mut result,
                );

                // Transfers the targets to the result with the specified size
                XGetWindowProperty(
                    self.display,
                    self.window,
                    self.prop_id,
                    0,
                    bytes_left as i64 * mem::size_of::<Atom>() as i64,
                    False,
                    XA_ATOM,
                    &mut return_type_id,
                    &mut return_format,
                    &mut returned_items,
                    &mut bytes_left,
                    &mut result,
                );

                // Atom is represented as a c_ulong (u64)
                let result = mem::transmute::<_, *mut u64>(result);

                let targets = (0..returned_items)
                    .map(|i| {
                        // Result is a pointer to C Atoms. The offset is used to
                        // get to the next atom. The returned_items guarantee
                        // that only valid atoms will be accessed.
                        let atom: Atom = *result.offset(i as isize) as Atom;
                        let atom_name = XGetAtomName(self.display, atom);
                        let name = CString::from_raw(atom_name);
                        return (name.into_string().unwrap(), atom);
                    })
                    .collect::<HashMap<String, Atom>>();

                return Some(targets);
            }

            None
        }
    }

    /// Fetches the data stored in the clipboard according to the `target_id` which
    /// represents the target format the selection needs to be converted.
    pub fn get_clipboard(
        &self,
        clipboard_id: Atom,
        target_id: Atom,
        event: &mut XEvent,
    ) -> Option<String> {
        unsafe {
            let incr_id = XInternAtom(self.display, CString::new("INCR").unwrap().as_ptr(), 0);

            XConvertSelection(
                self.display,
                clipboard_id,
                target_id,
                self.prop_id,
                self.window,
                CurrentTime,
            );

            loop {
                XNextEvent(self.display, event);

                if event.type_ == SelectionNotify {
                    break;
                }
            }

            if event.selection.property != 0 {
                let mut return_type_id: Atom = mem::uninitialized();
                let mut return_format: c_int = 0;
                let mut returned_items: c_ulong = 0;
                let mut bytes_left: c_ulong = 0;
                let mut result: *mut c_uchar = mem::uninitialized();

                // Used to get the size and the type of the selection
                XGetWindowProperty(
                    self.display,
                    self.window,
                    self.prop_id,
                    0,
                    0,
                    False,
                    AnyPropertyType as u64,
                    &mut return_type_id,
                    &mut return_format,
                    &mut returned_items,
                    &mut bytes_left,
                    &mut result,
                );

                // Copying large buffer is not currently implemented
                // @TODO: Work with incr_id
                if return_type_id != incr_id {
                    XGetWindowProperty(
                        self.display,
                        self.window,
                        self.prop_id,
                        0,
                        bytes_left as i64 * mem::size_of::<c_char>() as i64,
                        False,
                        AnyPropertyType as u64,
                        &mut return_type_id,
                        &mut return_format,
                        &mut returned_items,
                        &mut bytes_left,
                        &mut result,
                    );

                    let data = CString::from_raw(result as *mut c_char);
                    let data = data.to_str().unwrap();
                    return Some(data.to_owned());
                };
            };
        }
        None
    }
}

impl ClipboardFunctions for Clipboard {
    /// Creates a new instance of the clipboard.
    /// 
    /// Connects to the XServer and creates a unmapped window for requesting data
    /// from the owner of the selection.
    fn new() -> Result<Self, &'static str> {
        let display = unsafe { XOpenDisplay(std::ptr::null()) };

        if display.is_null() {
            return Err("Could not connect to XServer");
        }

        let window = unsafe {
            XCreateSimpleWindow(
                display,
                XDefaultRootWindow(display),
                -10,
                -10,
                1,
                1,
                0,
                0,
                0,
            )
        };

        let prop_id =
            unsafe { XInternAtom(display, CString::new("XSEL_DATA").unwrap().as_ptr(), False) };

        Ok(Clipboard {
            display,
            window,
            prop_id,
        })
    }

    /// Watches the clipboard for changes and calls the callback function with
    /// the clipboard data when the content changes. It depends on the XFixes
    /// extension to request the XServer to notify the window whenever the selection
    /// changes. 
    //  Based on the stackoverflow answer: https://stackoverflow.com/a/44992967
    fn watch_clipboard(&self, callback: &ClipboardSink) {
        unsafe {
            let clipboard_id =
                XInternAtom(self.display, CString::new("CLIPBOARD").unwrap().as_ptr(), 0);
            let mut event_base = mem::uninitialized();
            let mut error_base = mem::uninitialized();
            let mut event: XEvent = mem::uninitialized();

            // Constant variables from the XFixes' header file
            #[allow(non_snake_case)]
            let XFixesSetSelectionOwnerNotifyMask = (1 as c_long) << 0;
            #[allow(non_snake_case)]
            let XFixesSelectionNotify = 0;

            if XFixesQueryExtension(self.display, &mut event_base, &mut error_base) == 0 {
                panic!("Could not use XFixes extenion");
            }

            XFixesSelectSelectionInput(
                self.display,
                self.window,
                clipboard_id,
                XFixesSetSelectionOwnerNotifyMask as u64,
            );

            loop {
                XNextEvent(self.display, &mut event);

                if event.type_ == event_base + XFixesSelectionNotify {
                    let targets = self.get_targets();

                    if targets.is_some() {
                        let targets = targets.unwrap();
                        let target_id = targets
                            .get("text/html")
                            .or_else(|| targets.get("UTF8_STRING"))
                            .or_else(|| targets.get("TEXT"));
                        if target_id.is_some() {
                            let clipboard_data =
                                self.get_clipboard(clipboard_id, *target_id.unwrap(), &mut event);

                            if clipboard_data.is_some() {
                                // Add extra metadata such as the clipboard owner
                                // and when the selection was copied from the owner
                                let owner = XGetSelectionOwner(self.display, clipboard_id);
                                let mut owner_title: *mut c_char = mem::uninitialized();
                                XFetchName(self.display, owner, &mut owner_title);
                                let owner_title =
                                    CString::from_raw(owner_title).to_str().unwrap().to_string();
                                let created_at = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("Oops went back in time")
                                    .as_secs();
                                let clipboard_data = if targets.get("text/html").is_some() {
                                    ClipboardData::Html {
                                        owner: owner_title,
                                        content: clipboard_data.unwrap(),
                                        created_at,
                                    }
                                } else {
                                    ClipboardData::UnicodeText {
                                        owner: owner_title,
                                        content: clipboard_data.unwrap(),
                                        created_at,
                                    }
                                };
                                if let Err(e) = callback(clipboard_data) {
                                    eprintln!("Error while trying to save the file {}", e);
                                };
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Call the drop function to destroy the window and close the connection to the XServer.
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            XDeleteProperty(self.display, self.window, self.prop_id);
            XDestroyWindow(self.display, self.window);
            XCloseDisplay(self.display);
        }
    }
}
