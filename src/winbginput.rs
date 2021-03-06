// Code inspired by https://social.msdn.microsoft.com/Forums/en-US/4f731541-1819-4391-bd66-d026b629b786/detect-keypress-in-the-background

use std::convert::TryInto;

use winapi::{
    ctypes::*,
    shared::{minwindef::*, windef::*},
    um::{
        processthreadsapi::GetCurrentProcess,
        psapi::EnumProcessModules,
        winuser::{
            CallNextHookEx, SetWindowsHookExA, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
            WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
        },
    },
};

struct InputHandler {
    hook_handle: HHOOK,
    key_handler: fn(i32) -> (),
}

static mut GLOBAL_IH: Option<InputHandler> = None;

fn get_main_module_handle() -> Option<HINSTANCE> {
    let module_handle_box: Box<[HMODULE; 1024]> = Box::new([std::ptr::null_mut(); 1024]);
    let module_handle_arr: *mut [HMODULE; 1024] = Box::into_raw(module_handle_box);
    let mut cbs_needed: DWORD = 0;

    unsafe {
        let process_handle = GetCurrentProcess();
        if process_handle == std::ptr::null_mut() {
            drop(Box::from_raw(module_handle_arr));
            return None;
        }

        let enum_proc_success = EnumProcessModules(
            process_handle,
            module_handle_arr.cast::<HMODULE>(),
            std::mem::size_of::<[HMODULE; 1024]>().try_into().unwrap(),
            &mut cbs_needed,
        );

        let output = match enum_proc_success {
            0 => {
                println!("Failed enumerating modules");
                None
            }
            // The module at index 0 is always the main module, which will
            // always exist if we succeeded
            _ => Some((*module_handle_arr)[0]).clone(),
        };

        // Manually drop pointer because it is no longer Boxed
        drop(Box::from_raw(module_handle_arr));

        return output;
    }
}

pub fn init(key_handler: fn(i32) -> ()) -> () {
    unsafe {
        match &mut GLOBAL_IH {
            Some(existing_ih) => {
                existing_ih.key_handler = key_handler;
            }
            None => {
                let mut ih = InputHandler {
                    hook_handle: std::ptr::null_mut(),
                    key_handler: key_handler,
                };

                let main_module_handle = get_main_module_handle();
                if main_module_handle.is_none() {
                    println!("Error getting module handle for current process! Time to debug...");
                    return;
                }

                println!("Creating hook");
                ih.hook_handle = SetWindowsHookExA(
                    WH_KEYBOARD_LL,
                    Some(hook_fn),
                    main_module_handle.unwrap(),
                    0,
                );

                GLOBAL_IH = Some(ih);
            }
        }
    }
}

pub fn destroy() {
    unsafe {
        match &GLOBAL_IH {
            None => (),
            Some(ih) => {
                println!("Unhooking registered hook");
                UnhookWindowsHookEx(ih.hook_handle);
                GLOBAL_IH = None;
            }
        }
    }
}

#[no_mangle]
extern "system" fn hook_fn(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    println!("code: {} w_param: {} l_param: {}", code, w_param, l_param);
    unsafe {
        if let Some(ih) = &GLOBAL_IH {
            if code >= 0 {
                match w_param as UINT {
                    WM_KEYDOWN | WM_SYSKEYDOWN => {
                        let kb_struct_ptr: *const isize = &l_param;
                        let kb_struct_ptr: *const KBDLLHOOKSTRUCT = kb_struct_ptr.cast();
                        if let Some(kb_struct) = kb_struct_ptr.as_ref() {
                            let vk_code = kb_struct.vkCode;
                            let scan_code = kb_struct.scanCode;
                            let flags = kb_struct.flags;
                            let time = kb_struct.time;
                            println!("vk_code: {} scan_code: {}", vk_code, scan_code);
                            println!("flags: {} time: {}", flags, time);
                            (ih.key_handler)(vk_code as i32);
                        } else {
                            println!("Failed to dereference pointer...");
                        }
                    }
                    _ => (),
                }
            }
            return CallNextHookEx(ih.hook_handle, code, w_param, l_param);
        } else {
            return 0;
        }
    }
}
