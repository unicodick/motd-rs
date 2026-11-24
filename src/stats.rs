use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
};

pub(crate) const PATH: &str = "/data/attempts.jsonl";

pub(crate) struct Stats {
    file: File,
    len: u64,
    count: i32,
    partial: bool,
}

impl Stats {
    pub(crate) fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        file.try_lock()?;
        let mut reader = BufReader::new(&file);
        let mut line = Vec::new();
        let mut len = 0;
        let mut count = 0i32;

        while reader.read_until(b'\n', &mut line)? != 0 {
            if !line.ends_with(b"\n") {
                file.set_len(len)?;
                break;
            }
            len += line.len() as u64;
            count = count.saturating_add(1);
            line.clear();
        }

        Ok(Self {
            file,
            len,
            count,
            partial: false,
        })
    }

    pub(crate) fn count(&self) -> i32 {
        self.count
    }

    pub(crate) fn add(&mut self, player: &str) -> io::Result<i32> {
        let mut line = serde_json::to_vec(player)?;
        line.push(b'\n');

        if self.partial {
            self.file.set_len(self.len)?;
        }
        self.partial = true;
        self.file.write_all(&line)?;
        self.partial = false;
        self.len += line.len() as u64;
        self.count = self.count.saturating_add(1);
        Ok(self.count)
    }
}
