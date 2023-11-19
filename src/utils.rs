use std::ptr;
use winsafe::{EmptyClipboard, HGLOBAL, HWND, SetClipboardData};
use winsafe::co::{CF, GMEM};
use winsafe::prelude::{Handle, kernel_Hglobal, user_Hwnd};

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