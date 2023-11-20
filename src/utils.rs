use std::ffi::c_void;
use std::iter::once;
use std::os::windows::prelude::OsStrExt;
use std::ptr;
use log::info;
use windows::Win32::Foundation::BOOL;
use winsafe::{EmptyClipboard, HGLOBAL, HWND, SetClipboardData};
use winsafe::co::{CF, GMEM};
use winsafe::prelude::{Handle, kernel_Hglobal, user_Hwnd};
use once_cell::sync::Lazy;
use windows::Win32::Graphics::Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE;

pub(crate) fn copy_text_to_clipboard(hwnd: &HWND, text: &str) -> anyhow::Result<()> {
    let _open = hwnd.OpenClipboard()?;
    EmptyClipboard()?;

    let mut wstr = text.encode_utf16().collect::<Vec<u16>>();
    wstr.push(0); // terminate with \0

    let hg = HGLOBAL::GlobalAlloc(Some(GMEM::MOVEABLE), wstr.len() * std::mem::size_of::<u16>())?;
    {
        let dst = hg.GlobalLock()?;
        unsafe { ptr::copy_nonoverlapping(wstr.as_ptr(), dst.as_ptr() as _, wstr.len()) };
    }

    unsafe { let _ = SetClipboardData(CF::UNICODETEXT, hg.ptr() as _)?; }

    Ok(())
}

pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(once(0)).collect()
}




pub(crate) fn try_set_dark_mode(hwnd: &HWND) -> anyhow::Result<()> {

    let theme = encode_wide("DarkMode_Explorer");
    let ptr_theme = windows::core::PCWSTR::from_raw(theme.as_ptr());
    let w_hwnd = windows::Win32::Foundation::HWND(hwnd.ptr() as isize);
    let status = unsafe { windows::Win32::UI::Controls::SetWindowTheme(w_hwnd, ptr_theme, windows::core::PCWSTR::null()) };

    info!("try_dark_mode: {status:?}");

    if status.is_ok() {
        if set_dark_mode_for_window(w_hwnd, true) {
            return Ok(())
        }else {
            return Err(anyhow::Error::msg("Could not set dark mode :(".to_owned()));
        }
    }

    Ok(())

}

macro_rules! get_function {
    ($lib:expr, $func:ident) => {
        get_function_impl(
            concat!($lib, '\0'),
            concat!(stringify!($func), '\0'),
        )
        .map(|f| unsafe { std::mem::transmute::<*const _, $func>(f) })
    };
}

fn set_dark_mode_for_window(hwnd: windows::Win32::Foundation::HWND, is_dark_mode: bool) -> bool {

    // This is a simple implementation of support for Windows Dark Mode,
    // which is inspired by the solution in https://github.com/ysc3839/win32-darkmode

    // Uses Windows undocumented API SetWindowCompositionAttribute,

    type SetWindowCompositionAttribute =
    unsafe extern "system" fn( windows::Win32::Foundation::HWND, *mut WINDOWCOMPOSITIONATTRIBDATA) -> BOOL;

    #[allow(clippy::upper_case_acronyms)]
    type WINDOWCOMPOSITIONATTRIB = u32;
    const WCA_USEDARKMODECOLORS: WINDOWCOMPOSITIONATTRIB = 26;

    #[allow(non_snake_case)]
    #[allow(clippy::upper_case_acronyms)]
    #[repr(C)]
    struct WINDOWCOMPOSITIONATTRIBDATA {
        Attrib: WINDOWCOMPOSITIONATTRIB,
        pvData: *mut c_void,
        cbData: usize,
    }

    static SET_WINDOW_COMPOSITION_ATTRIBUTE: Lazy<Option<SetWindowCompositionAttribute>> =
        Lazy::new(|| get_function!("user32.dll", SetWindowCompositionAttribute));

    let res =  if let Some(set_window_composition_attribute) = *SET_WINDOW_COMPOSITION_ATTRIBUTE {
        unsafe {
            // SetWindowCompositionAttribute needs a bigbool (i32), not bool.
            let mut is_dark_mode_bigbool = BOOL::from(is_dark_mode);

            let mut data = WINDOWCOMPOSITIONATTRIBDATA {
                Attrib: WCA_USEDARKMODECOLORS,
                pvData: &mut is_dark_mode_bigbool as *mut _ as _,
                cbData: std::mem::size_of_val(&is_dark_mode_bigbool) as _,
            };

            let status = set_window_composition_attribute(hwnd, &mut data);

            status != BOOL(0)
        }
    } else {
        false
    };

   unsafe {
       let value = BOOL(1);
       let size = std::mem::size_of::<BOOL>();
       let res = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, std::ptr::addr_of!(value) as _, size as u32);
       info!("DwmSetWindowAttribute: {res:?}")
   };

    res
}


pub(super) fn get_function_impl(library: &str, function: &str) -> Option<*const c_void> {
    assert_eq!(library.chars().last(), Some('\0'));
    assert_eq!(function.chars().last(), Some('\0'));

    // Library names we will use are ASCII so we can use the A version to avoid string conversion.
    let module = unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryA(library.as_ptr()) };
    if module == 0 {
        return None;
    }

    unsafe { windows::core::imp::GetProcAddress(module, function.as_ptr()) }.map(|function_ptr| function_ptr as _)
}
