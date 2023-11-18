use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::ops::{Bound, RangeBounds};
use log::debug;
use crate::{settings, SETTINGS};

#[derive(Debug, Copy, Clone)]
struct LastBound {
    pub left: u64,
    pub right: u64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LineChunk {
    fst_line: u64,
    lst_line: u64,
    left_offset: u64,
    right_offset: u64,
}

#[derive(Debug)]
pub struct LineBasedFileView {
    reader: BufReader<File>,
    lines: Vec<LineChunk>,
    line_cache: Vec<String>,
    last_bounds: Option<LastBound>,
    def_cache_size: u64,
}

impl LineBasedFileView {
    pub fn new(file_path: String) -> anyhow::Result<Self> {
        let file = File::open(&file_path)?;
        let mut reader = BufReader::new(file);
        let mut lines: Vec<LineChunk> = vec![];

        let mut chunk: LineChunk = LineChunk {
            lst_line: 0,
            fst_line: 0,
            right_offset: 0,
            left_offset: 0,
        };

        let def_cache_size = if let Ok(settings) = SETTINGS.read() {
            settings.cache_size
        } else {
            settings::DEF_CACHE_RANGE
        };

        let chunk_size = def_cache_size;

        let mut str_buf = String::new();
        while let Ok(bytes_read) = reader.read_line(&mut str_buf) {
            if bytes_read == 0 {
                break;
            }
            chunk.lst_line = chunk.lst_line + 1;
            chunk.right_offset = reader.stream_position().unwrap();

            if chunk.lst_line % chunk_size == 0 {
                // get current stream pos & push
                lines.push(chunk.clone());

                // reset chunk for next page
                chunk.left_offset = chunk.right_offset;
                chunk.fst_line = chunk.lst_line;
            }

            str_buf.clear();
        }

        if let Some(last) = lines.last() {
            if last.ne(&chunk) {
                lines.push(chunk);
            }
        } else { // file is small enough to fit into one page
            lines.push(chunk);
        }

        Ok(Self {
            lines,
            reader,
            line_cache: vec![],
            last_bounds: None,
            def_cache_size,
        })
    }

    pub fn page_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_count(&self) -> u64 {
        if let Some(page) = self.lines.last() {
            page.lst_line
        } else {
            0
        }
    }

    pub fn get_line(&mut self, index: u64) -> Result<String, String> {
        if let Some(last_bounds) = &self.last_bounds {
            if last_bounds.left <= index && index < last_bounds.right {
                return if let Some(line) = self.line_cache.get((index - last_bounds.left) as usize)
                {
                    Ok(line.clone())
                } else {
                    Err(format!("ERROR READING LINE {index} with ERR: NOT FOUND"))
                };
            }
        }

        let def_cache_range = self.def_cache_size;

        let left = if index > def_cache_range {
            index - def_cache_range
        } else {
            0
        };
        match self.cache_lines(left..=u64::min(index + def_cache_range, self.line_count())) {
            Ok(_) => self.get_line(index),
            Err(err) => Err(format!("ERROR READING LINE {index} with ERR: {err}")),
        }
    }

    fn cache_lines(&mut self, r: impl RangeBounds<u64>) -> anyhow::Result<()> {
        let left = match r.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            Bound::Unbounded => 0,
        };

        let right = match r.end_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i - 1,
            Bound::Unbounded => (self.lines.len() - 1) as u64,
        };

        let left_page_index = left / self.def_cache_size;
        let right_page_index = right / self.def_cache_size;

        let left_page = *self.lines.get(left_page_index as usize)
            .unwrap_or_else(|| self.lines.first().unwrap_or(&LineChunk {
                fst_line: 0,
                lst_line: 0,
                left_offset: 0,
                right_offset: 0,
            }));

        let right_page = *self.lines.get(right_page_index as usize)
            .unwrap_or_else(|| self.lines.last().unwrap_or(&LineChunk {
                fst_line: 0,
                lst_line: 0,
                left_offset: 0,
                right_offset: 0,
            }));

        self.last_bounds = Some(LastBound { left: left_page.fst_line, right: right_page.lst_line });

        self.reader.seek(SeekFrom::Start(left_page.left_offset))?;

        let buf_length = (right_page.right_offset - left_page.left_offset) as usize;
        let mut buf = vec![0; buf_length];

        self.reader.read_exact(buf.as_mut_slice())?;

        let res = BufReader::new(buf.as_slice());
        self.line_cache = res.lines().map(|l| l.unwrap()).collect();

        debug!("LEFT_PAGE = {left_page:?} || RIGHT_PAGE = {right_page:?} || R.START = {:?} || R.END = {:?} || SELF.LASTBOUNDS = {:?} || CACHELEN = {}", r.start_bound(), r.end_bound(), &self.last_bounds, self.line_cache.len());

        Ok(())
    }
}
