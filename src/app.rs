use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_int, c_long, c_ulong};
use x11::xlib::*;

pub struct App {
    display: *mut Display,
    window: Window,
    prop_id: Atom,
}

extern "C" {
    fn XFixesSelectSelectionInput(_4: *mut Display, _3: Window, _2: Atom, _1: c_ulong);
    fn XFixesQueryExtension(_3: *mut Display, _2: *mut c_int, _1: *mut c_int) -> c_int;
}

impl App {
    pub fn new() -> Result<Self, &'static str> {
        let display = unsafe { XOpenDisplay(std::ptr::null()) };

        if display.is_null() {
            return Err("Could not open XDisplay");
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

        Ok(App {
            display,
            window,
            prop_id,
        })
    }

    pub fn get_targets(&self) -> HashMap<String, Atom> {
        let mut targets = HashMap::default();
        let mut event: XEvent = unsafe { std::mem::uninitialized() };
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
                use std::os::raw::*;

                let mut return_type_id: Atom = std::mem::uninitialized();
                let mut return_format: c_int = 0;
                let mut returned_items: c_ulong = 0;
                let mut bytes_left: c_ulong = 0;
                let mut result: *mut c_uchar = std::mem::uninitialized();

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
                    bytes_left as i64 * std::mem::size_of::<Atom>() as i64,
                    False,
                    XA_ATOM,
                    &mut return_type_id,
                    &mut return_format,
                    &mut returned_items,
                    &mut bytes_left,
                    &mut result,
                );

                let result = std::mem::transmute::<_, *mut u64>(result);

                for i in 0..returned_items {
                    let atom: Atom = *result.offset(i as isize) as Atom;
                    let atom_name = XGetAtomName(self.display, atom);
                    let name = CString::from_raw(atom_name);
                    targets.insert(name.into_string().unwrap(), atom);
                }
            }

            targets
        }
    }

    pub fn watch_clipboard(&self) {
        unsafe {
            let clipboard_id =
                XInternAtom(self.display, CString::new("CLIPBOARD").unwrap().as_ptr(), 0);
            let mut event_base = std::mem::uninitialized();
            let mut error_base = std::mem::uninitialized();
            let mut event: XEvent = std::mem::uninitialized();

            let XFixesSetSelectionOwnerNotifyMask = (1 as c_long) << 0;
            let XFixesSelectionNotify = 0;

            assert!(XFixesQueryExtension(self.display, &mut event_base, &mut error_base) != 0);
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
                    let target_id = targets
                        .get("text/html")
                        .or_else(|| targets.get("UTF8_STRING"));
                    if target_id.is_some() {
                        let incr_id =
                            XInternAtom(self.display, CString::new("INCR").unwrap().as_ptr(), 0);

                        XConvertSelection(
                            self.display,
                            clipboard_id,
                            *target_id.unwrap(),
                            self.prop_id,
                            self.window,
                            CurrentTime,
                        );

                        loop {
                            XNextEvent(self.display, &mut event);

                            if event.type_ == SelectionNotify {
                                break;
                            }
                        }

                        if event.selection.property != 0 {
                            use std::os::raw::*;

                            let mut return_type_id: Atom = std::mem::uninitialized();
                            let mut return_format: c_int = 0;
                            let mut returned_items: c_ulong = 0;
                            let mut bytes_left: c_ulong = 0;
                            let mut result: *mut c_uchar = std::mem::uninitialized();

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
                                    bytes_left as i64 * std::mem::size_of::<c_char>() as i64,
                                    False,
                                    AnyPropertyType as u64,
                                    &mut return_type_id,
                                    &mut return_format,
                                    &mut returned_items,
                                    &mut bytes_left,
                                    &mut result,
                                );
                                println!("{:?}", CString::from_raw(result as *mut c_char));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            XDeleteProperty(self.display, self.window, self.prop_id);
            XDestroyWindow(self.display, self.window);
            XCloseDisplay(self.display);
        }
    }
}
