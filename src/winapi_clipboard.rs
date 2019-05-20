use crate::common::ClipboardFunctions;
use crate::common::ClipboardSink;
use failure::{format_err, Error};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::Mutex;
use winapi::ctypes::wchar_t;
use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HWND, POINT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winbase::{GlobalLock, GlobalSize, GlobalUnlock};
use winapi::um::winuser::{
    AddClipboardFormatListener, CloseClipboard, CreateWindowExW, DefWindowProcW, DispatchMessageW,
    EnumClipboardFormats, GetClipboardData, GetMessageW, IsClipboardFormatAvailable, OpenClipboard,
    PostQuitMessage, RegisterClassW, RegisterClipboardFormatW, RemoveClipboardFormatListener,
    TranslateMessage, CF_UNICODETEXT, CS_OWNDC, CW_USEDEFAULT, HWND_MESSAGE, MSG,
    WM_CLIPBOARDUPDATE, WM_DESTROY, WNDCLASSW, WS_MINIMIZE,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum ClipboardData {
    Html(String, String),
    #[serde(rename = "text")]
    UnicodeText(String),
}

pub struct Clipboard {
    callback: Option<ClipboardSink>,
}

impl Clipboard {
    pub fn new() -> Self {
        Clipboard { callback: None }
    }

    pub fn get_formats() -> HashSet<u32> {
        let mut formats = HashSet::new();
        unsafe {
            if OpenClipboard(null_mut()) != 0 {
                let mut format = EnumClipboardFormats(0);
                loop {
                    if format == 0 {
                        break;
                    }
                    formats.insert(format);
                    format = EnumClipboardFormats(format);
                }
            }
        }
        formats
    }

    pub fn get_clipboard(&mut self) -> Result<ClipboardData, Error> {
        let formats = Clipboard::get_formats();
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return Err(format_err!("An error occured while opening the clipboard"));
            }
            // check for CF_CLIPBOARD
            let html_wide: Vec<u16> = OsStr::new("HTML Format")
                .encode_wide()
                .chain(once(0))
                .collect();
            let cf_html = RegisterClipboardFormatW(html_wide.as_ptr());
            let clipboard_data = if formats.contains(&cf_html) {
                let data = GetClipboardData(cf_html);
                let data = GlobalLock(data);
                let data_str = std::ffi::CString::from_raw(data as *mut i8).into_string()?;
                let c = RE.captures(&data_str).expect("chose wrong regex");
                let fragment = data_str
                    .get(c[4].parse::<usize>()?..c[5].parse::<usize>()?)
                    .expect("chose wrong regex..fix coming soon")
                    .to_string();
                let source_url = (&c[6]).to_string();
                GlobalUnlock(data);
                Ok(ClipboardData::Html(source_url, fragment))
            } else if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
                let data = GetClipboardData(CF_UNICODETEXT);
                let data = GlobalLock(data);
                let len = GlobalSize(data) / std::mem::size_of::<wchar_t>() - 1;
                let v = Vec::from_raw_parts(data as *mut u16, len, len);
                GlobalUnlock(data);
                Ok(ClipboardData::UnicodeText(String::from_utf16(&v)?))
            } else {
                Err(format_err!("Non-text format not available"))
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
            let data = CLIPBOARD.lock().unwrap().get_clipboard().unwrap();
            CLIPBOARD.lock().unwrap().callback.as_ref().unwrap().0(data).unwrap();
            1
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub struct Window(HWND);

impl Window {
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
                return Err(format_err!("Failed to register class"));
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
                return Err(format_err!("An error occurred while creating the window"));
            }

            Ok(hwnd)
        }
    }
}

impl ClipboardFunctions for Window {
    fn new() -> Result<Self, Error> {
        let hwnd = Window::create_window()?;
        Ok(Window(hwnd))
    }

    fn get_clipboard(&self) -> Result<ClipboardData, Error> {
        let data = CLIPBOARD.lock().unwrap().get_clipboard()?;
        Ok(data)
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

            if AddClipboardFormatListener(self.0) != 0 {
                while GetMessageW(&mut msg as *mut MSG, self.0, 0, 0) != 0 {
                    TranslateMessage(&msg as *const MSG);
                    DispatchMessageW(&msg as *const MSG);
                }
                RemoveClipboardFormatListener(self.0);
            }
        }
    }
}

lazy_static! {
    static ref CLIPBOARD: Mutex<Clipboard> = Mutex::new(Clipboard { callback: None });
    static ref RE: Regex = Regex::new(
        r#"(?x)
        Version:(\d+.\d+)\r\n
        StartHTML:(\d+)\r\n
        EndHTML:(\d+)\r\n
        StartFragment:(\d+)\r\n
        EndFragment:(\d+)\r\n
        SourceURL:(\S+)\r\n
        "#
    )
    .unwrap();
    // @TODO: Implement regex for the other version
    // static ref RE_EX: Regex = Regex::new(
    //     r#"(?x)
    //     Version:(\d+.\d+)\r\n
    //     StartHTML:(\d+)\r\n
    //     EndHTML:(\d+)\r\n
    //     StartFragment:(\d+)\r\n
    //     EndFragment:(\d+)\r\n
    //     StartSelection:(\d+)\r\n
    //     EndSelection:(\d+)\r\n
    //     SourceURL:(\S+)\r\n
    //     "#
    // )
    // .unwrap();
}
