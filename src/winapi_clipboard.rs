use crate::common::{ClipboardData, ClipboardFunctions, ClipboardSink, ClipboardTargets};
use failure::{bail, format_err, Error};
use lazy_static::lazy_static;
use regex::Regex;
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
    DispatchMessageW, EnumClipboardFormats, GetClipboardData, GetForegroundWindow, GetMessageW,
    GetWindowTextW, IsClipboardFormatAvailable, OpenClipboard, PostQuitMessage, RegisterClassW,
    RegisterClipboardFormatW, RemoveClipboardFormatListener, TranslateMessage, CF_BITMAP, CF_DIB,
    CF_DIBV5, CF_DIF, CF_DSPBITMAP, CF_DSPENHMETAFILE, CF_DSPMETAFILEPICT, CF_DSPTEXT,
    CF_ENHMETAFILE, CF_GDIOBJFIRST, CF_GDIOBJLAST, CF_HDROP, CF_LOCALE, CF_METAFILEPICT,
    CF_OEMTEXT, CF_OWNERDISPLAY, CF_PALETTE, CF_PENDATA, CF_PRIVATEFIRST, CF_PRIVATELAST, CF_RIFF,
    CF_SYLK, CF_TEXT, CF_TIFF, CF_UNICODETEXT, CF_WAVE, CS_OWNDC, CW_USEDEFAULT, HWND_MESSAGE, MSG,
    WM_CLIPBOARDUPDATE, WM_DESTROY, WNDCLASSW, WS_MINIMIZE, GetClipboardFormatNameW
};

pub struct Clipboard {
    callback: Option<ClipboardSink>,
}

impl Clipboard {
    fn get_formats() -> Result<HashSet<u32>, Error> {
        let mut formats = HashSet::new();
        unsafe {
            let mut format = EnumClipboardFormats(0);
            loop {
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

    fn get_clipboard() -> Result<ClipboardData, Error> {
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                bail!(io::Error::last_os_error());
            }
            let formats = Clipboard::get_formats()?;

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
                GlobalUnlock(data);
                Ok(ClipboardData::new((fragment, owner, source_url)))
            } else if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
                let data = GetClipboardData(CF_UNICODETEXT);
                if data.is_null() {
                    bail!(io::Error::last_os_error());
                }
                let data = GlobalLock(data);
                if data.is_null() {
                    bail!(io::Error::last_os_error());
                }
                let data_len = GlobalSize(data) / std::mem::size_of::<wchar_t>() - 1;
                let raw_data = Vec::from_raw_parts(data as *mut u16, data_len, data_len);
                GlobalUnlock(data);
                let data = String::from_utf16(&raw_data)?;
                Ok(ClipboardData::new((data, owner)))
            } else {
                bail!("Non-text format not available")
            };
            CloseClipboard();
            clipboard_data
        }
    }
}

#[allow(dead_code)]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            let data = Clipboard::get_clipboard();
            if data.is_ok() {
                CLIPBOARD.lock().unwrap().callback.as_ref().unwrap().0(data.unwrap()).unwrap();
            } else {
                let err_msg = data.unwrap_err();
                println!("An error occured: {}", err_msg);
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

fn create_window() -> Result<HWND, Error> {
    let class_name: Vec<u16> = OsStr::new("Clipoard Rust")
        .encode_wide()
        .chain(once(0))
        .collect();

    unsafe {
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
}

pub struct ClipboardOwner(HWND);

impl ClipboardFunctions for ClipboardOwner {
    fn new() -> Result<Self, Error> {
        let hwnd = create_window()?;
        Ok(ClipboardOwner(hwnd))
    }

    fn get_targets(&self) -> Result<ClipboardTargets, Error> {
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                bail!(io::Error::last_os_error());
            }
            let formats = Clipboard::get_formats()?;
            CloseClipboard();
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

    fn get_clipboard(&self) -> Result<ClipboardData, Error> {
        Clipboard::get_clipboard()
    }

    fn watch_clipboard(&self, callback: &ClipboardSink) {
        unsafe {
            CLIPBOARD.lock().unwrap().callback = Some(callback.clone());
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
            RemoveClipboardFormatListener(self.0);
        }
    }
}

impl Drop for ClipboardOwner {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

lazy_static! {
    static ref CLIPBOARD: Mutex<Clipboard> = Mutex::new(Clipboard { callback: None });
    static ref HTML_RE: Regex = Regex::new(
        r#"(?x)
        StartFragment:(?P<start>\d+)\s+
        EndFragment:(?P<end>\d+)\s+
        (?:SourceURL:(?P<url>\S++))?
        "#
    )
    .unwrap();
}
