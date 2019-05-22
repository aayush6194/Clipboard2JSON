use crate::common::{ClipboardData, ClipboardFunctions, ClipboardSink, ClipboardTargets};
use failure::{bail, format_err, Error};
use lazy_static::lazy_static;
use regex::Regex;
use scopeguard::defer;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io;
use std::iter::once;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::Mutex;
use winapi::ctypes::wchar_t;
use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HWND, POINT};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winbase::{GlobalLock, GlobalSize, GlobalUnlock};
use winapi::um::winuser::{
    AddClipboardFormatListener, CloseClipboard, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW,
    GetForegroundWindow, GetMessageW, GetWindowTextW, IsClipboardFormatAvailable, OpenClipboard,
    PostQuitMessage, RegisterClassW, RegisterClipboardFormatW, RemoveClipboardFormatListener,
    TranslateMessage, CF_BITMAP, CF_DIB, CF_DIBV5, CF_DIF, CF_DSPBITMAP, CF_DSPENHMETAFILE,
    CF_DSPMETAFILEPICT, CF_DSPTEXT, CF_ENHMETAFILE, CF_GDIOBJFIRST, CF_GDIOBJLAST, CF_HDROP,
    CF_LOCALE, CF_METAFILEPICT, CF_OEMTEXT, CF_OWNERDISPLAY, CF_PALETTE, CF_PENDATA,
    CF_PRIVATEFIRST, CF_PRIVATELAST, CF_RIFF, CF_SYLK, CF_TEXT, CF_TIFF, CF_UNICODETEXT, CF_WAVE,
    CS_OWNDC, CW_USEDEFAULT, HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE, WM_DESTROY, WNDCLASSW,
    WS_MINIMIZE,
};

/// Gets a hashset of all the data formats available on the clipboard.
///
/// If the format is a standard clipboard format then its name and description
/// can be found at [MDN](https://docs.microsoft.com/en-us/windows/desktop/dataxchg/standard-clipboard-formats).
/// If the format is a registered format, `GetClipboardFormatNameW` can be used
/// to get its name.  
/// The clipboard must be *opened* by calling the `OpenClipboard` function before
/// calling this function.  
/// More information about the underlying WinAPI function can be found at [MDN]
/// (https://docs.microsoft.com/en-us/windows/desktop/api/winuser/nf-winuser-enumclipboardformats)
fn get_formats() -> Result<HashSet<u32>, Error> {
    let mut formats = HashSet::new();
    unsafe {
        let mut format = EnumClipboardFormats(0);
        loop {
            // format is 0 if all the formats were successfully read
            // or if it is an error
            if format == 0 {
                match io::Error::last_os_error().raw_os_error() {
                    Some(e) => {
                        if e != ERROR_SUCCESS as i32 {
                            bail!(io::Error::last_os_error())
                        }
                    }
                    None => bail!("Unknown error"),
                }
                break;
            }
            formats.insert(format);
            format = EnumClipboardFormats(format);
        }
    }
    Ok(formats)
}

/// Gets the text-based data stored in the clipboard.
///
/// This function returns the data in HTML Format, if possible, or gets in the
/// UTF-16 format. The function `OpenClipboard` with the NULL pointer sets the
/// clipboard owner to none so the `GetForegroundWindow` is used to get the active
/// window which is set as the owner of the clipboard. There is a `GetClipboardOwner`
/// function available but it did not seem to work consistently.
fn get_clipboard() -> Result<ClipboardData, Error> {
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            bail!(io::Error::last_os_error());
        }

        defer! {{
            CloseClipboard();
        }}

        let formats = get_formats()?;

        // check for CF_CLIPBOARD
        let html_wide: Vec<u16> = OsStr::new("HTML Format")
            .encode_wide()
            .chain(once(0))
            .collect();
        let cf_html = RegisterClipboardFormatW(html_wide.as_ptr());
        let owner = GetForegroundWindow();
        let owner = if owner.is_null() {
            None
        } else {
            let mut raw_data: [u16; 255] = mem::uninitialized();
            let data_len = GetWindowTextW(owner, raw_data.as_mut_ptr(), 255) as usize;
            let owner_title = String::from_utf16_lossy(&raw_data[0..data_len]);
            Some(owner_title)
        };
        let clipboard_data = if formats.contains(&cf_html) {
            let data = GetClipboardData(cf_html);
            if data.is_null() {
                bail!(io::Error::last_os_error());
            }
            let data = GlobalLock(data);
            defer! {{
                GlobalUnlock(data);
            }}

            if data.is_null() {
                bail!(io::Error::last_os_error());
            }
            let data_str = std::ffi::CString::from_raw(data as *mut i8).into_string()?;
            let captures = HTML_RE.captures(&data_str).ok_or(format_err!(
                "An error occured while using regex on the HTML clipboard data"
            ))?;
            let fragment = data_str
                .get(captures[1].parse::<usize>()?..captures[2].parse::<usize>()?)
                .ok_or(format_err!(
                    "An error occured while trying to get the start and end fragments"
                ))?
                .to_string();
            let source_url = captures
                .name("url")
                .map_or(None, |url| Some(url.as_str().to_string()));
            Ok(ClipboardData::new((fragment, owner, source_url)))
        } else if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
            let data = GetClipboardData(CF_UNICODETEXT);
            if data.is_null() {
                bail!(io::Error::last_os_error());
            }
            let data = GlobalLock(data);
            defer! {{
                GlobalUnlock(data);
            }}

            if data.is_null() {
                bail!(io::Error::last_os_error());
            }
            let data_len = GlobalSize(data) / std::mem::size_of::<wchar_t>() - 1;
            let raw_data = Vec::from_raw_parts(data as *mut u16, data_len, data_len);
            let data = String::from_utf16(&raw_data)?;
            Ok(ClipboardData::new((data, owner)))
        } else {
            bail!("Non-text format not available")
        };
        clipboard_data
    }
}

/// The callback function called by Windows in response to incoming message queues.
/// This function is used to listen for `WM_CLIPBOARDUPDATE` events and calls the
/// callback function stored in a global variable by getting the new data from
/// the clipboard. The function prints an error message when something wrong happens
/// like a non-text  format is pasted to the clipboard and it cannot be converted
/// to text-based format.
#[allow(dead_code)]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            let data = get_clipboard();
            if data.is_ok() {
                CLIPBOARD.lock().unwrap().as_ref().unwrap().0(data.unwrap()).unwrap();
            } else {
                let err_msg = data.unwrap_err();
                eprintln!("An error occured: {}", err_msg);
            }
            1
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Creates a windowless window. While the window is not needed to get the data
/// from the clipboard, it is used to listen for the clipboard update events
/// and call the proper callback function.
/// This function is marked unsafe because it returns a raw pointer to the handle
/// of the newly created window. The window pointed by the handle must be destroyed
/// before dropping the value.
unsafe fn create_window() -> Result<HWND, Error> {
    let class_name: Vec<u16> = OsStr::new("Clipoard Rust")
        .encode_wide()
        .chain(once(0))
        .collect();

    let wc = WNDCLASSW {
        style: CS_OWNDC,
        lpfnWndProc: Some(wnd_proc),
        hInstance: GetModuleHandleW(null_mut()),
        lpszClassName: class_name.as_ptr(),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null_mut(),
    };

    if RegisterClassW(&wc) == 0 {
        bail!(io::Error::last_os_error());
    }

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        WS_MINIMIZE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        HWND_MESSAGE,
        null_mut(),
        GetModuleHandleW(null_mut()),
        null_mut(),
    );

    if hwnd.is_null() {
        bail!(io::Error::last_os_error());
    }

    Ok(hwnd)
}

/// Holds the pointer to the handle of the windowless window. The handle is used
/// for listening and responding to the message queue.
pub struct ClipboardOwner(HWND);

impl ClipboardOwner {
    /// Creates a new instance of the struct by creating a new windowless window.
    /// Note that the callback function is not passed at this pointer but instead
    /// when calling the watch_clipboard()` functiion.
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let hwnd = create_window()?;
            Ok(ClipboardOwner(hwnd))
        }
    }
}

impl ClipboardFunctions for ClipboardOwner {
    /// Gets the list of all the clipboard formats along with their registered
    /// names. It compares against the list of all standard clipboard formats which
    /// can be found at [MDN](https://docs.microsoft.com/en-us/windows/desktop/dataxchg/standard-clipboard-formats).
    /// If the clipboard is a registered format then it queries for its name. This
    /// is needed for the HTML Format which is a registered format.
    fn get_targets(&self) -> Result<ClipboardTargets, Error> {
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                bail!(io::Error::last_os_error());
            }
            defer! {{
                CloseClipboard();
            }}
            let formats = get_formats()?;
            let formats = formats.iter().fold(HashMap::new(), |mut map, format| {
                let name = match *format {
                    CF_BITMAP => "CF_BITMAP".to_string(),
                    CF_DIB => "CF_DIB".to_string(),
                    CF_DIBV5 => "CF_DIBV5".to_string(),
                    CF_DIF => "CF_DIF".to_string(),
                    CF_DSPBITMAP => "CF_DSPBITMAP".to_string(),
                    CF_DSPENHMETAFILE => "CF_DSPENHMETAFILE".to_string(),
                    CF_DSPMETAFILEPICT => "CF_DSPMETAFILEPICT".to_string(),
                    CF_DSPTEXT => "CF_DSPTEXT".to_string(),
                    CF_ENHMETAFILE => "CF_ENHMETAFILE".to_string(),
                    CF_GDIOBJFIRST => "CF_GDIOBJFIRST".to_string(),
                    CF_GDIOBJLAST => "CF_GDIOBJLAST".to_string(),
                    CF_HDROP => "CF_HDROP".to_string(),
                    CF_LOCALE => "CF_LOCALE".to_string(),
                    CF_METAFILEPICT => "CF_METAFILEPICT".to_string(),
                    CF_OEMTEXT => "CF_OEMTEXT".to_string(),
                    CF_OWNERDISPLAY => "CF_OWNERDISPLAY".to_string(),
                    CF_PALETTE => "CF_PALETTE".to_string(),
                    CF_PENDATA => "CF_PENDATA".to_string(),
                    CF_PRIVATEFIRST => "CF_PRIVATEFIRST".to_string(),
                    CF_PRIVATELAST => "CF_PRIVATELAST".to_string(),
                    CF_RIFF => "CF_RIFF".to_string(),
                    CF_SYLK => "CF_SYLK".to_string(),
                    CF_TEXT => "CF_TEXT".to_string(),
                    CF_TIFF => "CF_TIFF".to_string(),
                    CF_UNICODETEXT => "CF_UNICODETEXT".to_string(),
                    CF_WAVE => "CF_WAVE".to_string(),
                    format => {
                        let mut v: [u16; 255] = mem::uninitialized();
                        let len = GetClipboardFormatNameW(format, v.as_mut_ptr(), 255) as usize;
                        String::from_utf16_lossy(&v[0..len])
                    }
                };
                map.insert(name, *format);
                map
            });
            Ok(ClipboardTargets::WINAPI(formats))
        }
    }

    /// Gets the clipboard data in a text-based format if possible. It tries to
    /// return the text in the HTML format if possible or returns it as the UTF-16
    /// Windows string.
    fn get_clipboard(&self) -> Result<ClipboardData, Error> {
        get_clipboard()
    }

    /// Adds the window to the clipboard format listener list, sets up the window
    /// to listen for events and stores the callback function in a global variable.
    fn watch_clipboard(&self, callback: &ClipboardSink) {
        unsafe {
            *CLIPBOARD.lock().unwrap() = Some(callback.clone());
            let mut msg = MSG {
                hwnd: self.0,
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: POINT { x: 0, y: 0 },
            };

            if AddClipboardFormatListener(self.0) == 0 {
                panic!(
                    "Could not add clipboard format listener {}",
                    io::Error::last_os_error()
                );
            }

            defer! {{
                RemoveClipboardFormatListener(self.0);
            }}

            loop {
                let ret = GetMessageW(&mut msg as *mut MSG, self.0, 0, 0);

                if ret == 0 {
                    break;
                } else if ret == -1 {
                    eprintln!(
                        "An error occured while retrieving message {}",
                        io::Error::last_os_error()
                    );
                }
                TranslateMessage(&msg as *const MSG);
                DispatchMessageW(&msg as *const MSG);
            }
        }
    }
}

impl Drop for ClipboardOwner {
    /// Calls the `DestroyWindow` function to destroy the windowless window
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

lazy_static! {
    /// Global variable which is used to store the callback function that gets
    /// called when the clipboard is updated. The callback needs to be a static
    /// global in order to be usable by the WndProc callback function. WndProc is
    /// an unsafe extern function which can neither be modified to accept the callback
    /// nor can it be wrapped inside a higher function where the callback is passed in
    /// as a paramter since an unsafe function cannot capture the paramter variables
    /// (closures cannot be unsafe).
    static ref CLIPBOARD: Mutex<Option<ClipboardSink>> = Mutex::new(None);
    /// Used for extracting the fields in the HTML Clipboard. The StartFragment
    /// and EndFragment is used to exactly extract the HTML Clipboard selection.
    /// The source url is optional since applications such as Electron-based
    /// applications can have the HTML Clipboard without the SourceURL.
    static ref HTML_RE: Regex = Regex::new(
        r#"(?x)
        StartFragment:(?P<start>\d+)\s+
        EndFragment:(?P<end>\d+)\s+
        (?:SourceURL:(?P<url>\S++))?
        "#
    )
    .unwrap();
}
