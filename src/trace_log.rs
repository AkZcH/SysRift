use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct TraceEvent {
    pub num: u64,
    pub name: String,
    pub args: [u64; 6],
    pub ret: i64,
    pub data: Option<Vec<u8>>,
}

pub struct TraceWriter {
    writer: BufWriter<File>,
}

impl TraceWriter {
    pub fn create(path: &str) -> Self {
        let file = File::create(path).expect("failed to create trace log");
        TraceWriter {
            writer: BufWriter::new(file),
        }
    }

    pub fn write_event(&mut self, event: &TraceEvent) {
        let json = serde_json::to_string(event).expect("serialize failed");
        writeln!(self.writer, "{}", json).expect("write failed");
    }
}

pub struct TraceReader {
    lines: std::io::Lines<BufReader<File>>,
}

impl TraceReader {
    pub fn open(path: &str) -> Self {
        let file = File::open(path).expect("failed to open trace log");
        TraceReader {
            lines: BufReader::new(file).lines(),
        }
    }
}

impl Iterator for TraceReader {
    type Item = TraceEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.lines.next()?.expect("read line failed");
        Some(serde_json::from_str(&line).expect("deserialize failed"))
    }
}
