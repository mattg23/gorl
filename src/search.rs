use crate::SETTINGS;
use bitpacking::{BitPacker, BitPacker8x};
use fltk::app;
use grep::regex::RegexMatcherBuilder;
use grep::searcher::sinks::UTF8;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use log::{debug, error, info};
use std::fs::File;
use std::rc::Rc;
use std::sync::RwLock;

use crate::common::{GorlMsg, WindowId};
use crate::lineview::LineBasedFileView;
use crate::main_window::MwMessage;

fn search_in_file(query: &str, path: &str) -> anyhow::Result<CompressedSearchResults> {
    let start = std::time::Instant::now();

    let matcher = RegexMatcherBuilder::default()
        .case_insensitive(true)
        .line_terminator(Some(b'\n'))
        .build(query)?;

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true)
        .build();

    let mut search_res = CompressedSearchResults::new();
    let mut buffer = Vec::with_capacity(256);

    searcher.search_path(
        matcher,
        path,
        UTF8(|lnum, _| {
            search_res.append_line_number(lnum as u32, &mut buffer);
            Ok(true)
        }),
    )?;

    if !buffer.is_empty() {
        search_res.finish(&mut buffer);
    }

    let took = start.elapsed();
    let mb = humansize::format_size(search_res.get_size(), humansize::WINDOWS);
    info!(
        "search_in_file:: #Res={} took= {}ms, bytes = {mb}",
        search_res.get_count(),
        took.as_millis()
    );
    Ok(search_res)
}

struct SearchResultPage {
    pub compressed_0_offset: usize,
    pub compressed_len: usize,
    pub num_bits: u8,
    pub len: usize,
}

struct CompressedSearchResults {
    bytes: Vec<u8>,
    pages: Vec<SearchResultPage>,
}

impl CompressedSearchResults {
    pub fn new() -> Self {
        Self {
            bytes: vec![0; 8192],
            pages: Vec::new(),
        }
    }
    const BLOCK_LEN: usize = BitPacker8x::BLOCK_LEN;

    pub fn get(&self, index: usize) -> Option<u32> {
        if index > self.get_count() {
            return None;
        }

        let page_idx = index / Self::BLOCK_LEN;

        if let Some(page) = self.pages.get(page_idx) {
            let mut decompressed = vec![0u32; Self::BLOCK_LEN];
            let bit_packer = BitPacker8x::new();

            bit_packer.decompress_strictly_sorted(
                None,
                &self.bytes
                    [page.compressed_0_offset..(page.compressed_0_offset + page.compressed_len)],
                &mut decompressed,
                page.num_bits,
            );

            decompressed.get(index % Self::BLOCK_LEN).copied()
        } else {
            None
        }
    }

    pub fn get_count(&self) -> usize {
        self.pages.iter().map(|p| p.len).sum()
    }

    pub fn get_size(&self) -> usize {
        let size_bytes =
            std::mem::size_of::<Vec<u8>>() + self.bytes.capacity() * std::mem::size_of::<u8>();
        let size_pages = std::mem::size_of::<Vec<SearchResultPage>>()
            + self.pages.capacity() * std::mem::size_of::<SearchResultPage>();
        size_bytes + size_pages
    }
    pub fn append_line_number(&mut self, line_number: u32, buffer: &mut Vec<u32>) {
        if buffer.len() == Self::BLOCK_LEN {
            self.compress_and_add_page(buffer, buffer.len());
            buffer.clear();
        }

        buffer.push(line_number);
    }

    pub fn finish(&mut self, buffer: &mut Vec<u32>) {
        let valid_len = buffer.len();

        if valid_len == Self::BLOCK_LEN {
            self.compress_and_add_page(buffer, valid_len);
        } else {
            while buffer.len() < Self::BLOCK_LEN {
                buffer.push(0);
            }

            self.compress_and_add_page(buffer, valid_len);
        }

        buffer.clear();
        self.bytes.truncate(
            self.pages
                .last()
                .map(|p| p.compressed_0_offset + p.compressed_len)
                .unwrap_or(0),
        );
    }

    fn compress_and_add_page(&mut self, data: &mut Vec<u32>, valid_len: usize) {
        let bit_packer = BitPacker8x::new();

        let last_offset_used = self
            .pages
            .last()
            .map(|p| p.compressed_0_offset + p.compressed_len)
            .unwrap_or(0);
        let num_bits: u8 = bit_packer.num_bits_strictly_sorted(None, data.as_slice());
        let max_space_used = 4 * Self::BLOCK_LEN;

        let new_len = if self.bytes.len() - last_offset_used >= max_space_used {
            self.bytes.len()
        } else {
            self.bytes.len() + 2 * max_space_used
        };

        self.bytes.resize(new_len, 0);

        let slice = self.bytes[last_offset_used..].as_mut();
        let written = bit_packer.compress_strictly_sorted(None, data.as_slice(), slice, num_bits);
        let page = SearchResultPage {
            compressed_len: written,
            compressed_0_offset: last_offset_used,
            num_bits,
            len: valid_len,
        };

        self.pages.push(page);
    }
}

type SearchResults = Rc<RwLock<Option<CompressedSearchResults>>>;

#[derive(Clone)]
pub(crate) struct SearchWindow {
    parent: WindowId,
    current_file: Rc<RwLock<Option<String>>>,
    transmitter: app::Sender<GorlMsg>,
    current_search_results: SearchResults,
    view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
}

impl SearchWindow {
    pub fn new(
        parent: WindowId,
        transmitter: app::Sender<GorlMsg>,
        view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
    ) -> Self {
        Self {
            parent,
            current_file: Rc::new(RwLock::new(None)),
            transmitter,
            view,
            current_search_results: Rc::new(RwLock::new(None)),
        }
    }

    pub fn set_file(&self, new_path: &str) {
        *self.current_file.write().unwrap() = Some(new_path.to_owned());
        info!("SEARCHWINDOW: set file to {new_path}");
    }
}
