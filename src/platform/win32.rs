// Win32 platform implementation

use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::framebuffer::Framebuffer;
use super::{Event, WindowError};

const WINDOW_CLASS_NAME: &str = "AbrashWindowClass\0";

/// Win32 window handle wrapper
pub struct Window {
    hwnd: HWND,
    width: u32,
    height: u32,
    is_open: bool,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        unsafe {
            let hinstance = GetModuleHandleW(null_mut());

            // Register window class
            if !register_window_class(hinstance) {
                return Err(WindowError::RegistrationFailed);
            }

            // Convert title to wide string
            let mut title_wide: Vec<u16> = title.encode_utf16().collect();
            title_wide.push(0); // Null terminator

            // Create window
            let hwnd = CreateWindowExW(
                0,
                WINDOW_CLASS_NAME.as_ptr() as *const u16,
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

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn poll_events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            while PeekMessageW(&mut msg, null_mut() as HWND, 0, 0, PM_REMOVE) != 0 {
                match msg.message {
                    WM_QUIT => {
                        self.is_open = false;
                        events.push(Event::Close);
                    }
                    _ => {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }

        events
    }

    pub fn blit_framebuffer(&self, framebuffer: &Framebuffer) {
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
                framebuffer.as_slice().as_ptr() as *const _,
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );

            ReleaseDC(self.hwnd, hdc);
        }
    }
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
