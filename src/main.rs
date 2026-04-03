#![windows_subsystem = "windows"]

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, CloseHandle};
use windows::Win32::Storage::FileSystem::{CopyFileW, GetTempPathW};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::System::Threading::{
    CreateProcessW, ExitProcess, GetCurrentProcessId, OpenProcess, TerminateProcess,
    CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION, PROCESS_TERMINATE, STARTUPINFOW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetKeyboardLayout, GetKeyboardLayoutList,
    VK_LMENU, VK_LSHIFT, VK_MENU, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    ACTIVATE_KEYBOARD_LAYOUT_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW,
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

/// Возвращает путь к текущему exe
fn get_exe_path() -> Vec<u16> {
    let mut buf = vec![0u16; 1024];
    loop {
        let len = unsafe { GetModuleFileNameW(None, &mut buf) };
        if len == 0 {
            break;
        }
        if len < buf.len() as u32 {
            buf.truncate(len as usize);
            break;
        }
        buf.resize(buf.len() * 2, 0);
    }
    buf
}

/// Проверяет, что путь начинается с temp
fn is_in_temp(path: &[u16]) -> bool {
    let mut temp_buf = vec![0u16; 1024];
    let temp_len = unsafe { GetTempPathW(Some(&mut temp_buf)) };
    if temp_len == 0 || temp_len >= temp_buf.len() as u32 {
        return false;
    }
    temp_buf.truncate(temp_len as usize);

    path.len() >= temp_buf.len()
        && path[..temp_buf.len()].eq(&temp_buf[..])
}

/// Завершить предыдущий экземпляр, запущенный из temp
fn kill_previous_instance() {
    unsafe {
        let current_pid = GetCurrentProcessId();
        let temp_path = {
            let mut buf = vec![0u16; 1024];
            let len = GetTempPathW(Some(&mut buf));
            if len > 0 && len < buf.len() as u32 {
                buf.truncate(len as usize);
                buf.push(b'\\' as u16);
                Some(buf)
            } else {
                None
            }
        };

        let temp_path = match temp_path {
            Some(p) => p,
            None => return,
        };

        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };

        if Process32FirstW(snapshot, &mut entry).is_err() {
            let _ = CloseHandle(snapshot);
            return;
        }

        loop {
            if entry.th32ProcessID != current_pid {
                let name_slice = &entry.szExeFile[..entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len())];

                // Проверяем, что процесс запущен из temp
                let is_temp_instance = temp_path.len() <= name_slice.len()
                    && name_slice[..temp_path.len()].eq(&temp_path[..]);

                if is_temp_instance {
                    let process_handle = OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID);
                    if let Ok(handle) = process_handle {
                        let _ = TerminateProcess(handle, 1);
                        let _ = CloseHandle(handle);
                    }
                }
            }

            if Process32NextW(snapshot, &mut entry).is_err() {
                break;
            }
        }

        let _ = CloseHandle(snapshot);
    }
}

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
                    // swap вернет старое значение. Если было false -> значит это реальное нажатие, а не залипание
                    let was_pressed = ALT_PRESSED.swap(true, Ordering::SeqCst);
                    if !was_pressed && SHIFT_PRESSED.load(Ordering::SeqCst) {
                        switch_layout();
                        return LRESULT(1);
                    }
                }
                if is_keyup {
                    ALT_PRESSED.store(false, Ordering::SeqCst);
                }
            }

            if vk_code == VK_LSHIFT.0 || vk_code == VK_RSHIFT.0 || vk_code == VK_SHIFT.0 {
                if is_keydown {
                    let was_pressed = SHIFT_PRESSED.swap(true, Ordering::SeqCst);
                    if !was_pressed && ALT_PRESSED.load(Ordering::SeqCst) {
                        switch_layout();
                        return LRESULT(1);
                    }
                }
                if is_keyup {
                    SHIFT_PRESSED.store(false, Ordering::SeqCst);
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
        let exe_path = get_exe_path();

        // Если запущены не из temp — копируем туда и перезапускаемся
        if !is_in_temp(&exe_path) {
            kill_previous_instance();

            // Формируем путь к копии в temp
            let mut temp_buf = vec![0u16; 1024];
            let temp_len = GetTempPathW(Some(&mut temp_buf));
            if temp_len == 0 || temp_len >= temp_buf.len() as u32 {
                return;
            }
            temp_buf.truncate(temp_len as usize);

            let mut dest_path = temp_buf.clone();
            let exe_name: Vec<u16> = "altshift.exe\0".encode_utf16().collect();
            dest_path.extend_from_slice(&exe_name[..exe_name.len() - 1]);
            dest_path.push(0); // null terminator

            // Копируем файл
            if CopyFileW(
                PCWSTR(exe_path.as_ptr()),
                PCWSTR(dest_path.as_ptr()),
                false,
            ).is_err() {
                return;
            }

            // Запускаем копию
            let mut cmd_line = dest_path.clone();
            let si = STARTUPINFOW {
                cb: std::mem::size_of::<STARTUPINFOW>() as u32,
                ..std::mem::zeroed()
            };
            let mut pi = PROCESS_INFORMATION::default();

            if CreateProcessW(
                None,
                PWSTR(cmd_line.as_mut_ptr()),
                None,
                None,
                false,
                CREATE_UNICODE_ENVIRONMENT,
                None,
                None,
                &si,
                &mut pi,
            ).is_ok() {
                // Копия запущена — завершаем оригинал
                ExitProcess(0);
            }

            return;
        }

        // Работаем из temp — обычный запуск
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
