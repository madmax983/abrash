//! Win32 platform backend.
//!
//! Handles window creation and framebuffer presentation on Windows.

// Win32 platform implementation

use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDC, RGBQUAD, ReleaseDC, SRCCOPY,
    StretchDIBits,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, HMENU, IDC_ARROW, LoadCursorW, MSG, PM_REMOVE, PeekMessageW,
    PostQuitMessage, RegisterClassW, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_QUIT, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

use super::{Event, WindowError};
use crate::framebuffer::Framebuffer;

const WINDOW_CLASS_NAME: &str = "AbrashWindowClass";

/// Win32 window handle wrapper
pub struct Win32Window {
    hwnd: HWND,
    width: u32,
    height: u32,
    is_open: bool,
}

impl Win32Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        if width > i32::MAX as u32 || height > i32::MAX as u32 || width == 0 || height == 0 {
            return Err(WindowError::CreationFailed);
        }

        unsafe {
            let hinstance = GetModuleHandleW(null_mut());

            // Encode class name to UTF-16 with null terminator
            let class_name_wide: Vec<u16> = WINDOW_CLASS_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            // Register window class
            if !register_window_class(hinstance, &class_name_wide) {
                return Err(WindowError::RegistrationFailed);
            }

            // Convert title to wide string
            let mut title_wide: Vec<u16> = title.encode_utf16().collect();
            title_wide.push(0); // Null terminator

            // Create window
            let hwnd = CreateWindowExW(
                0,
                class_name_wide.as_ptr(),
                title_wide.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                null_mut() as HWND,
                null_mut() as HMENU,
                hinstance,
                null_mut(),
            );

            if hwnd.is_null() {
                return Err(WindowError::CreationFailed);
            }

            Ok(Self {
                hwnd,
                width,
                height,
                is_open: true,
            })
        }
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.is_open
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn poll_events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            while PeekMessageW(&raw mut msg, null_mut() as HWND, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    self.is_open = false;
                    events.push(Event::Close);
                } else {
                    TranslateMessage(&raw const msg);
                    DispatchMessageW(&raw const msg);
                }
            }
        }

        events
    }

    pub fn blit_framebuffer(&mut self, framebuffer: &Framebuffer) {
        unsafe {
            let hdc = GetDC(self.hwnd);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: framebuffer.width() as i32,
                    biHeight: -(framebuffer.height() as i32), // Negative for top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            StretchDIBits(
                hdc,
                0,
                0,
                self.width as i32,
                self.height as i32,
                0,
                0,
                framebuffer.width() as i32,
                framebuffer.height() as i32,
                framebuffer.as_slice().as_ptr().cast(),
                &raw const bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );

            ReleaseDC(self.hwnd, hdc);

            // Block until the next vertical blank (real vsync via DWM compositor)
            DwmFlush();
        }
    }
}

unsafe fn register_window_class(hinstance: HINSTANCE, class_name: &[u16]) -> bool {
    if class_name.last() != Some(&0) {
        return false;
    }

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
        lpszClassName: class_name.as_ptr(),
    };

    unsafe { RegisterClassW(&raw const wc) != 0 }
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
