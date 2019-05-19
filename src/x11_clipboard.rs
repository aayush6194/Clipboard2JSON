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

pub struct Clipboard {
    display: *mut Display,
    window: Window,
    prop_id: Atom,
}

extern "C" {
    fn XFixesSelectSelectionInput(_4: *mut Display, _3: Window, _2: Atom, _1: c_ulong);
    fn XFixesQueryExtension(_3: *mut Display, _2: *mut c_int, _1: *mut c_int) -> c_int;
}

impl Clipboard {
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
            XSelectInput(self.display, self.window, SelectionNotify.into());
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

                let result = mem::transmute::<_, *mut u64>(result);

                let targets = (0..returned_items)
                    .map(|i| {
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

    fn watch_clipboard(&self, callback: &ClipboardSink) {
        unsafe {
            let clipboard_id =
                XInternAtom(self.display, CString::new("CLIPBOARD").unwrap().as_ptr(), 0);
            let mut event_base = mem::uninitialized();
            let mut error_base = mem::uninitialized();
            let mut event: XEvent = mem::uninitialized();

            #[allow(non_snake_case)]
            let XFixesSetSelectionOwnerNotifyMask = (1 as c_long) << 0;
            #[allow(non_snake_case)]
            let XFixesSelectionNotify = 0;

            if XFixesQueryExtension(self.display, &mut event_base, &mut error_base) == 0 {
                eprintln!("Could not use XFixes extenion");
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

impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            XDeleteProperty(self.display, self.window, self.prop_id);
            XDestroyWindow(self.display, self.window);
            XCloseDisplay(self.display);
        }
    }
}
