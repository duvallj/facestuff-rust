use std::convert::TryInto;

use winapi::{
    ctypes::*,
    shared::{minwindef::*, ntdef::*, windef::*},
    um::{
        libloaderapi::GetModuleFileNameA,
        processthreadsapi::GetCurrentProcess,
        psapi::EnumProcessModules,
        winuser::{CallNextHookEx, SetWindowsHookExA, UnhookWindowsHookEx, WH_KEYBOARD_LL},
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

    let filename_box: Box<[CHAR; MAX_PATH]> = Box::new([0; MAX_PATH]);
    let filename_arr: *mut [CHAR; MAX_PATH] = Box::into_raw(filename_box);

    let mut cbs_needed: DWORD = 0;

    unsafe {
        let process_handle = GetCurrentProcess();
        if process_handle == std::ptr::null_mut() {
            return None;
        }

        let enum_proc_success = EnumProcessModules(
            process_handle,
            module_handle_arr.cast::<HMODULE>(),
            std::mem::size_of::<[HMODULE; 1024]>().try_into().unwrap(),
            &mut cbs_needed,
        );

        if enum_proc_success > 0 {
            let hmodule_size: u32 = std::mem::size_of::<HMODULE>().try_into().unwrap();
            let num_modules = cbs_needed / hmodule_size;
            for i in 0..num_modules {
                println!("{}", i);
                let read_filename_success = GetModuleFileNameA(
                    (*module_handle_arr)[i as usize],
                    filename_arr.cast::<CHAR>(),
                    MAX_PATH.try_into().unwrap(),
                );
                if read_filename_success > 0 {
                    let c_str: &std::ffi::CStr =
                        std::ffi::CStr::from_ptr(filename_arr.cast::<CHAR>());
                    let str_slice: &str = c_str.to_str().unwrap();
                    println!("module name: {}", str_slice);
                } else {
                    println!("Failed reading module name");
                }
            }
        } else {
            println!("Failed enumerating modules");
        }

        // Copy HINSTANCE out of array before we are done
        drop(Box::from_raw(module_handle_arr));
        drop(Box::from_raw(filename_arr));
    }

    None
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
                // TODO: do the rest of the initialization, actually registering
                // See https://social.msdn.microsoft.com/Forums/en-US/4f731541-1819-4391-bd66-d026b629b786/detect-keypress-in-the-background
                let main_module_handle = get_main_module_handle();
                if main_module_handle.is_none() {
                    println!("Error getting module handle for current process! Time to debug...");
                    return;
                }
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
                UnhookWindowsHookEx(ih.hook_handle);
                GLOBAL_IH = None;
            }
        }
    }
}

#[no_mangle]
extern "system" fn hook_fn(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        if let Some(ih) = &GLOBAL_IH {
            // TODO: call key_handler with the keycode that we get from *lParam
            return CallNextHookEx(ih.hook_handle, code, w_param, l_param);
        } else {
            return 0;
        }
    }
}
