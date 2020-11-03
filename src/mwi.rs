// Code inspired by https://social.msdn.microsoft.com/Forums/en-US/4f731541-1819-4391-bd66-d026b629b786/detect-keypress-in-the-background
// This code doesn't work, having trouble reading actual keycode from l_param (mostly boilerplate until line 120)

use std::convert::TryInto;
use std::io;

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

static mut PRESSED: bool = false;

// This routine is necessary for our call to SetWindowsHookExA, need have a handle to our executable
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
                eprintln!("Failed enumerating modules");
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

                eprintln!("Getting handle to main module");
                let main_module_handle = get_main_module_handle();
                if main_module_handle.is_none() {
                    eprintln!("Error getting module handle for current process! Time to debug...");
                    return;
                }

                eprintln!("Creating hook");
                ih.hook_handle = SetWindowsHookExA(
                    WH_KEYBOARD_LL,
                    Some(hook_fn),
                    main_module_handle.unwrap(),
                    0,
                );

                GLOBAL_IH = Some(ih);
                eprintln!("Successfully created hook");
            }
        }
    }
}

pub fn destroy() {
    unsafe {
        match &GLOBAL_IH {
            None => (),
            Some(ih) => {
                eprintln!("Unhooking registered hook");
                UnhookWindowsHookEx(ih.hook_handle);
                GLOBAL_IH = None;
            }
        }
    }
}

#[no_mangle]
extern "system" fn hook_fn(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    // See https://docs.microsoft.com/en-us/previous-versions/windows/desktop/legacy/ms644985(v=vs.85)
    // for how Windows should be calling this method
    eprintln!("code: {} w_param: {} l_param: {}", code, w_param, l_param);
    unsafe {
        if let Some(ih) = &GLOBAL_IH {
            if code >= 0 {
                // Oddly enough, despite l_param not working, w_param works perfectly fine here
                match w_param as UINT {
                    WM_KEYDOWN | WM_SYSKEYDOWN => {
                        let kb_struct_ptr: *const isize = &l_param;
                        // I had thought l_param itself would be a pointer, but trying to dereference it with
                        /* let kb_struct_ptr = std::ptr::null::<KBDLLHOOKSTRUCT>().add(l_param as usize); */
                        // resulted in an invalid dereference.
                        let kb_struct_ptr: *const KBDLLHOOKSTRUCT = kb_struct_ptr.cast();
                        if let Some(kb_struct) = kb_struct_ptr.as_ref() {
                            // Still, the data read here is garbled, and I'm probably doing something wrong
                            let vk_code = kb_struct.vkCode;
                            let scan_code = kb_struct.scanCode;
                            let flags = kb_struct.flags;
                            let time = kb_struct.time;
                            eprintln!("vk_code: {} scan_code: {}", vk_code, scan_code);
                            eprintln!("flags: {} time: {}", flags, time);
                            (ih.key_handler)(vk_code as i32);
                        } else {
                            eprintln!("Failed to dereference pointer...");
                        }

                        PRESSED = true;
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

fn key_handler(code: i32) {
    println!("{}", code);
}

fn main() -> io::Result<()> {
    init(key_handler);

    unsafe {
        while !PRESSED {
            // pass
        }
    }

    destroy();

    Ok(())
}
