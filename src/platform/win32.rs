// Win32 platform implementation

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use std::ptr::null_mut;

const WINDOW_CLASS_NAME: &str = "AbrashWindowClass\0";

/// Win32 window handle wrapper
pub struct Window {
    // TODO: Add HWND and other Win32 handles in Task 13
}

unsafe fn register_window_class(hinstance: HINSTANCE) -> bool {
    let class_name = WINDOW_CLASS_NAME.as_ptr() as *const u16;

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: null_mut(),
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: null_mut(),
        lpszMenuName: null_mut(),
        lpszClassName: class_name,
    };

    unsafe { RegisterClassW(&wc) != 0 }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
