use std::collections::Bound;
use std::fs::{File};
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, stdin};
use std::ops::RangeBounds;

fn main() -> anyhow::Result<()> {
    let file_path = "D:\\EVI_Valdarno\\Prod\\Service\\Src\\Exec\\Service\\bin\\Debug\\logs\\Service_2023-01-24.0.log";

    let mut view = LineBasedFileView::new(file_path.to_string())?;

    println!("{:?}", view.get_lines(..));

    let mut input = String::new();
    stdin().read_line(&mut input)?;

    Ok(())
}

struct LineBasedFileView {
    file_path: String,
    reader: BufReader<File>,
    lines: Vec<u64>,
}

impl LineBasedFileView {
    pub fn new(file_path: String) -> anyhow::Result<Self> {
        let file = File::open(&file_path)?;
        let mut reader = BufReader::new(file);
        let mut lines: Vec<u64> = vec![0];

        let mut buf: [u8; 1024] = [0x0; 1024];

        while let Ok(read) = reader.read(&mut buf)  {
            if read == 0 {
                break;
            }

            for (i,c) in buf[0..read].iter().enumerate() {
                if *c == b'\n' {
                    lines.push(lines.last().unwrap_or(&0) + (i +1) as u64)
                }
            }
        }

        Ok(Self{
            lines,
            file_path,
            reader
        })
    }

    pub fn line_count(self) -> u64 {
        self.lines.len() as u64
    }

    pub fn get_lines(&mut self, r: impl RangeBounds<u64>) -> anyhow::Result<Vec<io::Result<String>>> {
        let left = match r.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i+1,
            Bound::Unbounded => 0
        };
        let left_offset = *self.lines.get(left as usize).unwrap_or_else(|| self.lines.first().unwrap_or(&0));

        let right = match r.end_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i-1,
            Bound::Unbounded => (self.lines.len() - 1) as u64
        };
        let right_offset = *self.lines.get(right as usize).unwrap_or_else(|| self.lines.last().unwrap_or(&0));

        self.reader.seek(SeekFrom::Start(left_offset))?;

        let buf_length = (right_offset - left_offset) as usize;
        let mut buf = Vec::with_capacity(buf_length);
        buf.resize(buf_length, 0);

        let bytes_read = self.reader.read(buf.as_mut_slice())?;

        let res = BufReader::new(&buf.as_slice()[..bytes_read]);
        let res = res.lines().collect();

        Ok(res)
    }
}


