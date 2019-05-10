use std::collections::HashSet;
use std::ffi::CString;
use x11::xlib::*;

pub struct App {
    display: *mut Display,
    window: Window,
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

        Ok(App { display, window })
    }

    pub fn get_targets(&self) -> HashSet<String> {
        let mut targets = HashSet::default();
        let mut event: XEvent = unsafe { std::mem::uninitialized() };
        let targets_id = unsafe {
            XInternAtom(
                self.display,
                CString::new("TARGETS").unwrap().as_ptr(),
                False,
            )
        };
        let prop_id = unsafe {
            XInternAtom(
                self.display,
                CString::new("XSEL_DATA").unwrap().as_ptr(),
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
                prop_id,
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
                    prop_id,
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
                    prop_id,
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
                    targets.insert(name.into_string().unwrap());
                }
            }

            XDeleteProperty(self.display, self.window, prop_id);

            targets
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            XDestroyWindow(self.display, self.window);
            XCloseDisplay(self.display);
        }
    }
}
