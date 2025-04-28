// use crate::SETTINGS;
// use bitpacking::{BitPacker, BitPacker8x};
// use flume::Sender;
// use grep::regex::RegexMatcherBuilder;
// use grep::searcher::sinks::UTF8;
// use grep::searcher::{BinaryDetection, SearcherBuilder};
// use log::{debug, error, info};
// use std::fs::File;
// use std::rc::Rc;
// use std::sync::RwLock;

// use crate::lineview::LineBasedFileView;
// use winsafe::co::{
//     BS, CHARSET, CLIP, COLOR, ES, FW, LVS, LVS_EX, OUT_PRECIS, PITCH, QUALITY, VK, WS,
// };
// use winsafe::gui::{Brush, Horz, ListViewOpts, Vert};
// use winsafe::msg::wm::SetFont;
// use winsafe::{co, gui, prelude::*, WString, HFONT, SIZE};

// use crate::main_window::MwMessage;

// fn search_in_file(query: &str, path: &str) -> anyhow::Result<CompressedSearchResults> {
//     let start = std::time::Instant::now();

//     let matcher = RegexMatcherBuilder::default()
//         .case_insensitive(true)
//         .line_terminator(Some(b'\n'))
//         .build(query)?;

//     let mut searcher = SearcherBuilder::new()
//         .binary_detection(BinaryDetection::quit(b'\x00'))
//         .line_number(true)
//         .build();

//     let mut search_res = CompressedSearchResults::new();
//     let mut buffer = Vec::with_capacity(256);

//     searcher.search_path(
//         matcher,
//         path,
//         UTF8(|lnum, _| {
//             search_res.append_line_number(lnum as u32, &mut buffer);
//             Ok(true)
//         }),
//     )?;

//     if !buffer.is_empty() {
//         search_res.finish(&mut buffer);
//     }

//     let took = start.elapsed();
//     let mb = humansize::format_size(search_res.get_size(), humansize::WINDOWS);
//     info!(
//         "search_in_file:: #Res={} took= {}ms, bytes = {mb}",
//         search_res.get_count(),
//         took.as_millis()
//     );
//     Ok(search_res)
// }

// struct SearchResultPage {
//     pub compressed_0_offset: usize,
//     pub compressed_len: usize,
//     pub num_bits: u8,
//     pub len: usize,
// }

// struct CompressedSearchResults {
//     bytes: Vec<u8>,
//     pages: Vec<SearchResultPage>,
// }

// impl CompressedSearchResults {
//     pub fn new() -> Self {
//         Self {
//             bytes: vec![0; 8192],
//             pages: Vec::new(),
//         }
//     }
//     const BLOCK_LEN: usize = BitPacker8x::BLOCK_LEN;

//     pub fn get(&self, index: usize) -> Option<u32> {
//         if index > self.get_count() {
//             return None;
//         }

//         let page_idx = index / Self::BLOCK_LEN;

//         if let Some(page) = self.pages.get(page_idx) {
//             let mut decompressed = vec![0u32; Self::BLOCK_LEN];
//             let bit_packer = BitPacker8x::new();

//             bit_packer.decompress_strictly_sorted(
//                 None,
//                 &self.bytes
//                     [page.compressed_0_offset..(page.compressed_0_offset + page.compressed_len)],
//                 &mut decompressed,
//                 page.num_bits,
//             );

//             decompressed.get(index % Self::BLOCK_LEN).copied()
//         } else {
//             None
//         }
//     }

//     pub fn get_count(&self) -> usize {
//         self.pages.iter().map(|p| p.len).sum()
//     }

//     pub fn get_size(&self) -> usize {
//         let size_bytes =
//             std::mem::size_of::<Vec<u8>>() + self.bytes.capacity() * std::mem::size_of::<u8>();
//         let size_pages = std::mem::size_of::<Vec<SearchResultPage>>()
//             + self.pages.capacity() * std::mem::size_of::<SearchResultPage>();
//         size_bytes + size_pages
//     }
//     pub fn append_line_number(&mut self, line_number: u32, buffer: &mut Vec<u32>) {
//         if buffer.len() == Self::BLOCK_LEN {
//             self.compress_and_add_page(buffer, buffer.len());
//             buffer.clear();
//         }

//         buffer.push(line_number);
//     }

//     pub fn finish(&mut self, buffer: &mut Vec<u32>) {
//         let valid_len = buffer.len();

//         if valid_len == Self::BLOCK_LEN {
//             self.compress_and_add_page(buffer, valid_len);
//         } else {
//             while buffer.len() < Self::BLOCK_LEN {
//                 buffer.push(0);
//             }

//             self.compress_and_add_page(buffer, valid_len);
//         }

//         buffer.clear();
//         self.bytes.truncate(
//             self.pages
//                 .last()
//                 .map(|p| p.compressed_0_offset + p.compressed_len)
//                 .unwrap_or(0),
//         );
//     }

//     fn compress_and_add_page(&mut self, data: &mut Vec<u32>, valid_len: usize) {
//         let bit_packer = BitPacker8x::new();

//         let last_offset_used = self
//             .pages
//             .last()
//             .map(|p| p.compressed_0_offset + p.compressed_len)
//             .unwrap_or(0);
//         let num_bits: u8 = bit_packer.num_bits_strictly_sorted(None, data.as_slice());
//         let max_space_used = 4 * Self::BLOCK_LEN;

//         let new_len = if self.bytes.len() - last_offset_used >= max_space_used {
//             self.bytes.len()
//         } else {
//             self.bytes.len() + 2 * max_space_used
//         };

//         self.bytes.resize(new_len, 0);

//         let slice = self.bytes[last_offset_used..].as_mut();
//         let written = bit_packer.compress_strictly_sorted(None, data.as_slice(), slice, num_bits);
//         let page = SearchResultPage {
//             compressed_len: written,
//             compressed_0_offset: last_offset_used,
//             num_bits,
//             len: valid_len,
//         };

//         self.pages.push(page);
//     }
// }

// type SearchResults = Rc<RwLock<Option<CompressedSearchResults>>>;

// #[derive(Clone)]
// pub(crate) struct SearchWindow {
//     wnd: gui::WindowModeless,
//     search_query_txt_box: gui::Edit,
//     search_results_list: gui::ListView,
//     search_button: gui::Button,
//     current_file: Rc<RwLock<Option<String>>>,
//     transmitter: Sender<MwMessage>,
//     current_search_results: SearchResults,
//     view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
// }

// impl SearchWindow {
//     pub fn new(
//         parent: &impl GuiParent,
//         transmitter: Sender<MwMessage>,
//         view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
//     ) -> Self {
//         let wnd = gui::WindowModeless::new(
//             parent,
//             gui::WindowModelessOpts {
//                 class_bg_brush: Brush::Color(COLOR::BACKGROUND),
//                 title: "GORL - Search".to_string(),
//                 style: gui::WindowMainOpts::default().style
//                     | WS::MINIMIZEBOX
//                     | WS::MAXIMIZEBOX
//                     | WS::SIZEBOX
//                     | WS::POPUPWINDOW,
//                 size: (600, 350),
//                 ..Default::default() // leave all other options as default
//             },
//         );

//         let search_button = gui::Button::new(
//             &wnd,
//             gui::ButtonOpts {
//                 height: 24,
//                 width: 150,
//                 text: " 🔍 Search".to_owned(),
//                 position: (420, 10),
//                 button_style: BS::DEFPUSHBUTTON | BS::PUSHBUTTON,
//                 resize_behavior: (Horz::Repos, Vert::None),
//                 ..Default::default()
//             },
//         );

//         let search_query_txt_box = gui::Edit::new(
//             &wnd,
//             gui::EditOpts {
//                 text: "".to_string(),
//                 position: (10, 10),
//                 width: 400,
//                 height: 24,
//                 edit_style: ES::LEFT | ES::NOHIDESEL | ES::AUTOHSCROLL,
//                 resize_behavior: (Horz::Resize, Vert::None),
//                 ..Default::default()
//             },
//         );

//         let search_results = gui::ListView::new(
//             &wnd,
//             ListViewOpts {
//                 position: (10, 44),
//                 size: (560, 256),
//                 columns: vec![("Line".to_string(), 128), ("Text".to_string(), 3200)],
//                 resize_behavior: (Horz::Resize, Vert::Resize),
//                 list_view_ex_style: LVS_EX::DOUBLEBUFFER | LVS_EX::FULLROWSELECT,
//                 list_view_style: LVS::REPORT | LVS::NOLABELWRAP | LVS::OWNERDATA,
//                 ..Default::default()
//             },
//         );

//         let mut new_self = Self {
//             wnd,
//             search_query_txt_box,
//             search_results_list: search_results,
//             search_button,
//             current_file: Rc::new(RwLock::new(None)),
//             transmitter,
//             current_search_results: Rc::new(RwLock::new(None)),
//             view,
//         };

//         new_self.events(); // attach our events
//         new_self
//     }

//     pub fn set_file(&self, new_path: &str) {
//         *self.current_file.write().unwrap() = Some(new_path.to_owned());
//         info!("SEARCHWINDOW: set file to {new_path}");
//     }

//     // extern "system" fn handle_edit_text_box(
//     //     h_wnd: winsafe::HWND,
//     //     u_msg: co::WM,
//     //     w_param: usize,
//     //     l_param: isize,
//     //     _u_id_subclass: usize,
//     //     dw_ref_data: usize,
//     // ) -> isize {
//     //     if u_msg == co::WM::KEYUP {
//     //         unsafe {
//     //             if VK::from_raw(w_param as u16) == VK::RETURN {
//     //                 debug!(
//     //                     "handle_edit_text_box::SubClassProcedure  {}, w_param={}, lParama={}",
//     //                     u_msg,
//     //                     VK::RETURN,
//     //                     l_param
//     //                 );
//     //                 let ptr = dw_ref_data as *const Self;
//     //                 (*ptr).search_button.trigger_click();
//     //                 (*ptr).search_query_txt_box.focus();
//     //             }
//     //         }
//     //     }
//     //     let wm_any = winsafe::msg::WndMsg::new(u_msg, w_param, l_param);
//     //     h_wnd.DefSubclassProc(wm_any)
//     // }

//     // extern "system" fn subclass_search_result_list_view(
//     //     h_wnd: winsafe::HWND,
//     //     u_msg: co::WM,
//     //     w_param: usize,
//     //     l_param: isize,
//     //     _u_id_subclass: usize,
//     //     dw_ref_data: usize,
//     // ) -> isize {
//     //     if u_msg == co::WM::KEYDOWN {
//     //         unsafe {
//     //             if VK::from_raw(w_param as u16) == VK::CHAR_C
//     //                 && winsafe::GetAsyncKeyState(VK::CONTROL)
//     //             {
//     //                 let is_shift_down = winsafe::GetAsyncKeyState(VK::SHIFT);

//     //                 let ptr = dw_ref_data as *const Self;

//     //                 let sel_count = (*ptr).search_results_list.items().selected_count();
//     //                 if 0 < sel_count
//     //                     && sel_count <= SETTINGS.read().unwrap().max_nb_of_lines_to_copy
//     //                 {
//     //                     let mut str_to_cpy = String::new();

//     //                     for sel_item in (*ptr).search_results_list.items().iter_selected() {
//     //                         if is_shift_down {
//     //                             str_to_cpy.push_str(sel_item.text(0).as_str());
//     //                             str_to_cpy.push_str(" | ");
//     //                         }
//     //                         str_to_cpy.push_str(sel_item.text(1).as_str());
//     //                     }

//     //                     match crate::utils::copy_text_to_clipboard(&h_wnd, str_to_cpy.as_str()) {
//     //                         Ok(_) => {
//     //                             info!("subclass_list_view::SubClassProcedure: clipboard data has been set!")
//     //                         }
//     //                         Err(e) => {
//     //                             error!("subclass_list_view::SubClassProcedure: could not set clipboard data: {e}")
//     //                         }
//     //                     }
//     //                 }
//     //             }

//     //             debug!(
//     //                 "subclass_list_view::SubClassProcedure {}, w_param={}, lParama={}",
//     //                 u_msg, w_param, l_param
//     //             );
//     //         }
//     //     }
//     //     let wm_any = winsafe::msg::WndMsg::new(u_msg, w_param, l_param);
//     //     h_wnd.DefSubclassProc(wm_any)
//     // }

//     // fn events(&mut self) {
//     //     self.wnd.on().wm_create({
//     //         let myself = self.clone();
//     //         move |_msg| {
//     //             info!("SEARCH WINDOW: WM_CREATE");
//     //             let _ = crate::utils::try_set_dark_mode(myself.wnd.hwnd());
//     //             if let Ok(settings) = SETTINGS.read() {
//     //                 let mut font = HFONT::CreateFont(
//     //                     SIZE::new(0, settings.font.size),
//     //                     0,
//     //                     0,
//     //                     FW::MEDIUM,
//     //                     settings.font.italic,
//     //                     false,
//     //                     false,
//     //                     CHARSET::DEFAULT,
//     //                     OUT_PRECIS::DEFAULT,
//     //                     CLIP::DEFAULT_PRECIS,
//     //                     QUALITY::DEFAULT,
//     //                     PITCH::FIXED,
//     //                     settings.font.name.as_str(),
//     //                 )?;

//     //                 myself.search_query_txt_box.set_font(&font);

//     //                 myself.search_results_list.hwnd().SendMessage(
//     //                     SetFont {
//     //                         hfont: font.leak(),
//     //                         redraw: true,
//     //                     }
//     //                     .as_generic_wm(),
//     //                 );

//     //                 unsafe {
//     //                     let _ = myself.search_query_txt_box.hwnd().SetWindowSubclass(
//     //                         Self::handle_edit_text_box,
//     //                         0,
//     //                         &myself as *const _ as _,
//     //                     );
//     //                 }
//     //                 unsafe {
//     //                     let _ = myself.search_results_list.hwnd().SetWindowSubclass(
//     //                         Self::subclass_search_result_list_view,
//     //                         0,
//     //                         &myself as *const _ as _,
//     //                     );
//     //                 }
//     //             }
//     //             Ok(0)
//     //         }
//     //     });

//         self.search_query_txt_box.on().en_update({
//             let myself = self.clone();
//             move || {
//                 let text = myself.search_query_txt_box.text();
//                 const ASCII_DELETE: char = '\u{7f}';
//                 if text.ends_with(ASCII_DELETE) {
//                     // ends in ASCII 127 == DELETE character

//                     let (i, _) = text
//                         .char_indices()
//                         .rfind(|(_, c)| c.ne(&ASCII_DELETE) && c.is_whitespace())
//                         .unwrap_or((0, 's'));

//                     let next = &text[0..i];
//                     myself.search_query_txt_box.set_text(next);
//                     myself
//                         .search_query_txt_box
//                         .set_selection(i as i32, i as i32);
//                 }
//                 Ok(())
//             }
//         });

//         self.search_results_list.on().nm_dbl_clk({
//             let myself = self.clone();
//             move |msg| {
//                 let index = msg.iItem;
//                 let lnum_str = myself.search_results_list.items().get(index as u32).text(0);

//                 if let Ok(num) = lnum_str.as_str().parse::<u64>() {
//                     debug!(
//                         "SEARCH WINDOW: USER DOUBLE CLICKED ON ITEM {index} => parse to line {num}"
//                     );

//                     myself.transmitter.send(MwMessage::JumpTo(num))?;
//                 }

//                 Ok(())
//             }
//         });

//         self.search_results_list.on().lvn_get_disp_info({
//             let myself = self.clone();
//             move |info| {
//                 if myself
//                     .current_search_results
//                     .read()
//                     .is_ok_and(|o| o.is_none())
//                 {
//                     return Ok(());
//                 }

//                 if info.item.mask.has(co::LVIF::TEXT) {
//                     let index = info.item.iItem as usize;
//                     let line_set = match myself.current_search_results.write() {
//                         Ok(guard) => {
//                             if guard.is_some() {
//                                 let results = guard.as_ref().unwrap();
//                                 if let Some(line) = results.get(index) {
//                                     let split = format!("{line}");

//                                     let text_to_set = if info.item.iSubItem == 0 {
//                                         // first col
//                                         WString::from_str(split)
//                                     } else if let Ok(mut lock_res) = myself.view.write() {
//                                         if let Some(view_ref) = lock_res.as_mut() {
//                                             if let Ok(actual_line) = view_ref.get_line((line - 1) as u64) {
//                                                 WString::from_str(actual_line)
//                                             } else {
//                                                 WString::from_str("GORL ERROR IN SEARCH: Line not found")
//                                             }
//                                         } else {
//                                             WString::from_str("GORL ERROR IN SEARCH: Could not get lock view ref mutably INNER")
//                                         }
//                                     } else {
//                                         WString::from_str("GORL ERROR IN SEARCH: Could not get lock view ref mutably INNER")
//                                     };

//                                     let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
//                                     let out_slice =
//                                         unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
//                                     out_slice
//                                         .iter_mut()
//                                         .zip(text_to_set.as_slice())
//                                         .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
//                                     Ok(())
//                                 } else {
//                                     Err(format!("Line not found with index {index} "))
//                                 }
//                             } else {
//                                 Err("No search results available".to_string())
//                             }
//                         }
//                         Err(error) => Err(format!("{error}")),
//                     };

//                     if line_set.is_err() {
//                         error!(
//                             "SeachWindow: ERROR SETTING ITEM TEXT {index} {:?}",
//                             line_set.unwrap_err()
//                         );
//                     }
//                 }

//                 Ok(())
//             }
//         });

//         self.search_button.on().bn_clicked({
//             let myself = self.clone();
//             move || {
//                 info!("SEARCH WINDOW: SEARCH CLICKED");
//                 if let Ok(lock_res) = myself.current_file.read() {
//                     if let Some(file) = lock_res.as_ref() {
//                         let query = myself.search_query_txt_box.text();
//                         match search_in_file(query.as_str(), file.as_str()) {
//                             Ok(search_results) => {
//                                 if let Ok(mut guard) = myself.current_search_results.write() {
//                                     let view = search_results;
//                                     let len = view.get_count();
//                                     *guard = Some(view);

//                                     myself.wnd.set_text(
//                                         format!(
//                                             "GORL - Search - #RES={} [{}]",
//                                             len,
//                                             myself.current_file.read().unwrap().as_ref().unwrap()
//                                         )
//                                         .as_str(),
//                                     );

//                                     info!("SEARCH WINDOW: SEARCH EXECUTED. #RES={}", len);
//                                     myself.search_results_list.items().delete_all();
//                                     myself
//                                         .search_results_list
//                                         .items()
//                                         .set_count(len as u32, None);
//                                     //}
//                                 } else {
//                                     error!("COULD NOT LOCK SearchWindow.current_search_results")
//                                 }
//                             }
//                             Err(err) => {
//                                 error!("SEARCH WINDOW: ERROR DURING SEARCH: {err}");
//                             }
//                         }
//                     }
//                 }
//                 Ok(())
//             }
//         })
//     }
// }
