use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

pub fn create_writer(path: &Path) -> Result<BufWriter<File>> {
    let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    Ok(BufWriter::new(file))
}

pub fn open_append_writer(path: &Path) -> Result<BufWriter<File>> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    Ok(BufWriter::new(file))
}

pub fn write_line<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *writer, value).context("serialize json")?;
    writer.write_all(b"\n").context("write newline")?;
    Ok(())
}

pub fn read_lines<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read line {}", idx + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str::<T>(&line)
            .with_context(|| format!("parse json at {}:{}", path.display(), idx + 1))?;
        out.push(value);
    }
    Ok(out)
}
