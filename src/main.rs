#![windows_subsystem = "windows"]

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetKeyboardLayout, GetKeyboardLayoutList,
    VK_LMENU, VK_LSHIFT, VK_MENU, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    ACTIVATE_KEYBOARD_LAYOUT_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, DispatchMessageW,
    GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, MSG, HHOOK,
    WM_INPUTLANGCHANGEREQUEST, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::Win32::UI::TextServices::HKL;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static ALT_PRESSED: AtomicBool = AtomicBool::new(false);
static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(true);
static CURRENT_LANG: AtomicU32 = AtomicU32::new(0x0409);

extern "system" fn hook_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && ENABLED.load(Ordering::SeqCst) {
            let kbd_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk_code = kbd_struct.vkCode as u16;

            let w = wparam.0 as u32;
            let is_keydown = w == WM_KEYDOWN || w == WM_SYSKEYDOWN;
            let is_keyup = w == WM_KEYUP || w == WM_SYSKEYUP;

            if vk_code == VK_LMENU.0 || vk_code == VK_RMENU.0 || vk_code == VK_MENU.0 {
                if is_keydown {
                    ALT_PRESSED.store(true, Ordering::SeqCst);
                }
                if is_keyup {
                    ALT_PRESSED.store(false, Ordering::SeqCst);
                }
            }

            if vk_code == VK_LSHIFT.0 || vk_code == VK_RSHIFT.0 || vk_code == VK_SHIFT.0 {
                if is_keydown {
                    SHIFT_PRESSED.store(true, Ordering::SeqCst);
                }
                if is_keyup {
                    SHIFT_PRESSED.store(false, Ordering::SeqCst);
                }
            }

            if is_keydown {
                let alt = ALT_PRESSED.load(Ordering::SeqCst);
                let shift = SHIFT_PRESSED.load(Ordering::SeqCst);

                if alt && shift {
                    switch_layout();
                    ALT_PRESSED.store(false, Ordering::SeqCst);
                    SHIFT_PRESSED.store(false, Ordering::SeqCst);
                    return LRESULT(1);
                }
            }
        }

        CallNextHookEx(HHOOK::default(), code, wparam, lparam)
    }
}

fn switch_layout() {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0 == 0 {
            return;
        }

        let mut gui_info = std::mem::zeroed::<windows::Win32::UI::WindowsAndMessaging::GUITHREADINFO>();
        gui_info.cbSize = std::mem::size_of::<windows::Win32::UI::WindowsAndMessaging::GUITHREADINFO>() as u32;

        let fg_thread_id = GetWindowThreadProcessId(fg_hwnd, None);
        let target_hwnd = if GetGUIThreadInfo(fg_thread_id, &mut gui_info).is_ok() && gui_info.hwndFocus.0 != 0 {
            gui_info.hwndFocus
        } else {
            fg_hwnd
        };

        let current_hkl = GetKeyboardLayout(fg_thread_id);
        let lang_id = (current_hkl.0 as usize) & 0xFFFF;
        CURRENT_LANG.store(lang_id as u32, Ordering::SeqCst);

        let mut layouts: Vec<HKL> = vec![HKL::default(); 16];
        let count = GetKeyboardLayoutList(Some(&mut layouts));

        if count > 0 {
            let current_pos = layouts.iter()
                .position(|hkl| hkl.0 == current_hkl.0);

            let next_hkl = match current_pos {
                Some(pos) => layouts[(pos + 1) % count as usize],
                None => layouts[0],
            };

            let _ = PostMessageW(
                target_hwnd,
                WM_INPUTLANGCHANGEREQUEST,
                WPARAM(0),
                LPARAM(next_hkl.0 as isize),
            );

            let _ = ActivateKeyboardLayout(next_hkl, ACTIVATE_KEYBOARD_LAYOUT_FLAGS(0));

            let new_lang = if lang_id == 0x0409 { 0x0419 } else { 0x0409 };
            CURRENT_LANG.store(new_lang, Ordering::SeqCst);
        }
    }
}

fn main() {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0 != 0 {
            let thread_id = GetWindowThreadProcessId(fg_hwnd, None);
            let hkl = GetKeyboardLayout(thread_id);
            let lang_id = (hkl.0 as usize) & 0xFFFF;
            CURRENT_LANG.store(lang_id as u32, Ordering::SeqCst);
        }

        let module = GetModuleHandleW(None).expect("Не удалось получить HMODULE");

        let hook = SetWindowsHookExW(
            windows::Win32::UI::WindowsAndMessaging::WH_KEYBOARD_LL,
            Some(hook_callback),
            module,
            0,
        )
        .expect("Не удалось установить хук клавиатуры");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook);
    }
}
