use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
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
enum ClipboardData {
    Html(String, String),
    #[serde(rename = "text")]
    UnicodeText(String),
}

struct Clipboard {
    html_regex: Regex,
    data: Option<ClipboardData>,
}

impl Clipboard {
    pub fn new() -> Self {
        let html_regex = Regex::new(
            r#"(?x)
                Version:(\d+.\d+)\r\n
                StartHTML:(\d+)\r\n
                EndHTML:(\d+)\r\n
                StartFragment:(\d+)\r\n
                EndFragment:(\d+)\r\n
                SourceURL:(\S+)\r\n
            "#,
        )
        .unwrap();

        Clipboard {
            html_regex,
            data: None,
        }
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

    pub fn get_clipboard(&mut self) {
        let formats = Clipboard::get_formats();
        unsafe {
            if OpenClipboard(null_mut()) != 0 {
                // check for CF_CLIPBOARD
                let html_wide: Vec<u16> = OsStr::new("HTML Format")
                    .encode_wide()
                    .chain(once(0))
                    .collect();
                let cf_html = RegisterClipboardFormatW(html_wide.as_ptr());
                if formats.contains(&cf_html) {
                    let data = GetClipboardData(cf_html);
                    let data = GlobalLock(data);
                    let data_str = std::ffi::CString::from_raw(data as *mut i8)
                        .into_string()
                        .unwrap();
                    let c = self.html_regex.captures(&data_str).unwrap();
                    let fragment = data_str
                        .get(c[4].parse::<usize>().unwrap()..c[5].parse::<usize>().unwrap())
                        .unwrap();
                    let source_url = &c[6];
                    self.data = Some(ClipboardData::Html(
                        source_url.to_string(),
                        fragment.to_string(),
                    ));
                    GlobalUnlock(data);
                } else if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
                    let data = GetClipboardData(CF_UNICODETEXT);
                    let data = GlobalLock(data);
                    let len = GlobalSize(data) / std::mem::size_of::<wchar_t>() - 1;
                    let v = Vec::from_raw_parts(data as *mut u16, len, len);
                    let data_str = ClipboardData::UnicodeText(String::from_utf16(&v).unwrap());
                    self.data = Some(data_str);
                    GlobalUnlock(data);
                }
                CloseClipboard();
            }
        }
    }
}

fn create_window() -> Result<HWND, &'static str> {
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
            return Err("Failed to register class");
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
            return Err("Error creating window!");
        }

        Ok(hwnd)
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
            let mut clipboard = Clipboard::new();
            clipboard.get_clipboard();
            1
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn monitor_clipboard() {
    let hwnd = create_window().unwrap();
    let mut msg = MSG {
        hwnd,
        message: 0,
        wParam: 0,
        lParam: 0,
        time: 0,
        pt: POINT { x: 0, y: 0 },
    };

    unsafe {
        if AddClipboardFormatListener(hwnd) != 0 {
            while GetMessageW(&mut msg as *mut MSG, hwnd, 0, 0) != 0 {
                TranslateMessage(&msg as *const MSG);
                DispatchMessageW(&msg as *const MSG);
            }
            RemoveClipboardFormatListener(hwnd);
        }
    }
}
