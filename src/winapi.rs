use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HWND, POINT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    AddClipboardFormatListener, CloseClipboard, CreateWindowExW, DefWindowProcW, DispatchMessageW,
    GetClipboardData, GetMessageW, IsClipboardFormatAvailable, OpenClipboard, PostQuitMessage,
    RegisterClassW, RemoveClipboardFormatListener, TranslateMessage, CF_TEXT, CS_OWNDC,
    CW_USEDEFAULT, HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE, WM_DESTROY, WNDCLASSW, WS_MINIMIZE,
};

pub fn get_clipboard() {
    unsafe {
        if OpenClipboard(null_mut()) != 0 {
            if IsClipboardFormatAvailable(CF_TEXT) != 0 {
                let a = GetClipboardData(CF_TEXT) as *mut i8;
                let b = std::ffi::CString::from_raw(a);
                println!("{:?}", b);
            }
            CloseClipboard();
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
            get_clipboard();
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
