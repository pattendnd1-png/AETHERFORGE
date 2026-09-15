#[cfg(target_os = "windows")]
mod imp {
    use crate::app::{AetherApp, DrawCommand};
    use aether_ui::Rgba;
    use std::ffi::{c_int, c_void};
    use std::ptr;
    use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
    use std::thread;
    use std::time::Duration;
    type Hwnd = *mut c_void;
    type Hinstance = *mut c_void;
    type Hicon = *mut c_void;
    type Hcursor = *mut c_void;
    type Hbrush = *mut c_void;
    type Hdc = *mut c_void;
    type Lresult = isize;
    type Wparam = usize;
    type Lparam = isize;
    #[repr(C)]
    struct WndClassW {
        style: u32,
        wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult>,
        cls_extra: c_int,
        wnd_extra: c_int,
        instance: Hinstance,
        icon: Hicon,
        cursor: Hcursor,
        background: Hbrush,
        menu_name: *const u16,
        class_name: *const u16,
    }
    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        wparam: Wparam,
        lparam: Lparam,
        time: u32,
        pt_x: i32,
        pt_y: i32,
        l_private: u32,
    }
    #[repr(C)]
    struct RectW {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct PaintStruct {
        hdc: Hdc,
        erase: i32,
        paint: RectW,
        restore: i32,
        inc_update: i32,
        reserved: [u8; 32],
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassW(c: *const WndClassW) -> u16;
        fn CreateWindowExW(
            ex: u32,
            class: *const u16,
            name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: Hwnd,
            menu: *mut c_void,
            inst: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        fn DefWindowProcW(h: Hwnd, m: u32, w: Wparam, l: Lparam) -> Lresult;
        fn ShowWindow(h: Hwnd, n: i32) -> i32;
        fn UpdateWindow(h: Hwnd) -> i32;
        fn PeekMessageW(m: *mut Msg, h: Hwnd, min: u32, max: u32, remove: u32) -> i32;
        fn TranslateMessage(m: *const Msg) -> i32;
        fn DispatchMessageW(m: *const Msg) -> Lresult;
        fn PostQuitMessage(code: i32);
        fn BeginPaint(h: Hwnd, p: *mut PaintStruct) -> Hdc;
        fn EndPaint(h: Hwnd, p: *const PaintStruct) -> i32;
        fn FillRect(dc: Hdc, r: *const RectW, b: Hbrush) -> i32;
        fn InvalidateRect(h: Hwnd, r: *const RectW, erase: i32) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> Hinstance;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateSolidBrush(color: u32) -> Hbrush;
        fn DeleteObject(obj: *mut c_void) -> i32;
        fn SetTextColor(dc: Hdc, color: u32) -> u32;
        fn SetBkMode(dc: Hdc, mode: i32) -> i32;
        fn TextOutW(dc: Hdc, x: i32, y: i32, s: *const u16, n: i32) -> i32;
    }
    static APP: AtomicPtr<AetherApp> = AtomicPtr::new(ptr::null_mut());
    static WIDTH: AtomicI32 = AtomicI32::new(1400);
    static HEIGHT: AtomicI32 = AtomicI32::new(900);
    const WM_DESTROY: u32 = 2;
    const WM_PAINT: u32 = 15;
    const WM_SIZE: u32 = 5;
    const WM_CHAR: u32 = 0x102;
    const WM_LBUTTONDOWN: u32 = 0x201;
    const WS_OVERLAPPEDWINDOW: u32 = 0x00cf0000;
    const WS_VISIBLE: u32 = 0x10000000;
    const PM_REMOVE: u32 = 1;
    pub fn run(app: &mut AetherApp) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            APP.store(app as *mut _, Ordering::SeqCst);
            let inst = GetModuleHandleW(ptr::null());
            let class = wide("AetherAIClass");
            let wc = WndClassW {
                style: 0,
                wnd_proc: Some(wnd_proc),
                cls_extra: 0,
                wnd_extra: 0,
                instance: inst,
                icon: ptr::null_mut(),
                cursor: ptr::null_mut(),
                background: ptr::null_mut(),
                menu_name: ptr::null(),
                class_name: class.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                return Err("RegisterClassW failed".into());
            }
            let title = wide("AetherAI — DragonGlass");
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                80,
                1440,
                900,
                ptr::null_mut(),
                ptr::null_mut(),
                inst,
                ptr::null_mut(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW failed".into());
            }
            ShowWindow(hwnd, 5);
            UpdateWindow(hwnd);
            let mut msg = std::mem::zeroed::<Msg>();
            while !app.should_quit {
                while PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == 0x12 {
                        app.should_quit = true;
                        break;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if app.tick() {
                    InvalidateRect(hwnd, ptr::null(), 0);
                }
                thread::sleep(Duration::from_millis(16));
            }
            APP.store(ptr::null_mut(), Ordering::SeqCst);
        }
        Ok(())
    }
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe extern "system" fn wnd_proc(h: Hwnd, m: u32, w: Wparam, l: Lparam) -> Lresult {
        let p = APP.load(Ordering::SeqCst);
        match m {
            WM_DESTROY => {
                if !p.is_null() {
                    (*p).should_quit = true
                }
                PostQuitMessage(0);
                0
            }
            WM_SIZE => {
                WIDTH.store((l as u32 & 0xffff) as i32, Ordering::Relaxed);
                HEIGHT.store(((l as u32 >> 16) & 0xffff) as i32, Ordering::Relaxed);
                0
            }
            WM_CHAR => {
                if !p.is_null() {
                    match w as u32 {
                        8 => (*p).backspace(),
                        13 => (*p).enter(false),
                        27 => (*p).should_quit = true,
                        c if c >= 32 => {
                            if let Some(ch) = char::from_u32(c) {
                                (*p).handle_text(&ch.to_string())
                            }
                        }
                        _ => {}
                    }
                    InvalidateRect(h, ptr::null(), 0);
                }
                0
            }
            WM_LBUTTONDOWN => {
                if !p.is_null() {
                    let x = (l as u32 & 0xffff) as i16 as i32;
                    let y = ((l as u32 >> 16) & 0xffff) as i16 as i32;
                    (*p).click(
                        x,
                        y,
                        WIDTH.load(Ordering::Relaxed),
                        HEIGHT.load(Ordering::Relaxed),
                    );
                    InvalidateRect(h, ptr::null(), 0);
                }
                0
            }
            WM_PAINT => {
                let mut ps = std::mem::zeroed::<PaintStruct>();
                let dc = BeginPaint(h, &mut ps);
                if !p.is_null() {
                    draw(
                        dc,
                        &(*p).render(
                            WIDTH.load(Ordering::Relaxed),
                            HEIGHT.load(Ordering::Relaxed),
                        ),
                    );
                }
                EndPaint(h, &ps);
                0
            }
            _ => DefWindowProcW(h, m, w, l),
        }
    }
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn draw(dc: Hdc, commands: &[DrawCommand]) {
        SetBkMode(dc, 1);
        for c in commands {
            match c {
                DrawCommand::Rect { rect, color, .. } => {
                    let b = CreateSolidBrush(bgr(*color));
                    let r = RectW {
                        left: rect.x,
                        top: rect.y,
                        right: rect.x + rect.w,
                        bottom: rect.y + rect.h,
                    };
                    FillRect(dc, &r, b);
                    DeleteObject(b);
                }
                DrawCommand::Text { x, y, text, color } => {
                    SetTextColor(dc, bgr(*color));
                    let ws: Vec<u16> = text.encode_utf16().collect();
                    TextOutW(dc, *x, *y - 14, ws.as_ptr(), ws.len() as i32);
                }
            }
        }
    }
    fn bgr(c: Rgba) -> u32 {
        (c.r as u32) | ((c.g as u32) << 8) | ((c.b as u32) << 16)
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

pub use imp::run;
