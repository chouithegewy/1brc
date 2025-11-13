use libc::{MAP_FAILED, mmap64, munmap};
use std::collections::BTreeMap;
use std::fs::File;
use std::num::ParseFloatError;
use std::os::fd::AsRawFd;
use std::str;
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::{f32, io};

fn bytes_to_float(bytes: &[u8]) -> Result<f32, ParseFloatError> {
    let s = unsafe { str::from_utf8_unchecked(bytes) };
    s.parse::<f32>()
}

fn find_next_newline(data: &[u8]) -> Vec<usize> {
    data.iter()
        .enumerate()
        .filter(|&(_, byte)| *byte == b'\n')
        .map(|(i, _)| i)
        .take(1)
        .collect()
}

// provide chunks on data for parallel processing that include full lines
fn chunker(size: usize, data: &[u8]) -> Vec<(usize, usize)> {
    let thread_count =
        thread::available_parallelism().expect("Failed to get available_parallelism");
    let chunk_size = size / thread_count;
    let mut start: usize = 0;
    let mut end = start + chunk_size;
    let mut chunks: Vec<(usize, usize)> = vec![];

    while end + 32 <= size {
        let newline_index = find_next_newline(&data[end..(end + 32usize)])[0];
        chunks.push((start, end + newline_index + 1));
        start = end + newline_index + 1;
        end = start + chunk_size;
    }

    chunks
}

fn chunk_processor<'a>(data: &'a [u8], tx: Sender<BTreeMap<&'a str, StationResult<'a>>>) {
    let mut station_results: BTreeMap<&'a str, StationResult<'a>> = BTreeMap::new();

    let mut semicolon_index = 0usize;
    let mut start_line_index = 0usize;
    let mut newline_index = 0usize;

    for (i, c) in data.iter().enumerate() {
        if c == &b';' {
            semicolon_index = i;
        } else if c == &b'\n' {
            newline_index = i;
            let station_name = byte_2_str(&data[start_line_index..semicolon_index]);
            start_line_index = i + 1;
            let station_value =
                bytes_to_float(&data[semicolon_index + 1..newline_index]).expect("Should be valid");

            station_results
                .entry(station_name)
                .and_modify(|e: &mut StationResult<'a>| {
                    e.count += 1;
                    e.sum += station_value;
                    if station_value < e.min {
                        e.min = station_value;
                    }
                    if station_value > e.max {
                        e.max = station_value;
                    }
                })
                .or_insert(StationResult::new(
                    station_name,
                    1,
                    station_value,
                    station_value,
                    station_value,
                ));
        }
    }

    tx.send(station_results).unwrap();
}

fn byte_2_str<'a>(data: &'a [u8]) -> &'a str {
    unsafe { str::from_utf8_unchecked(&data) }
}

#[derive(Debug)]
struct StationResult<'a> {
    name: &'a str,
    count: u32,
    sum: f32,
    min: f32,
    max: f32,
}

impl<'a> StationResult<'a> {
    fn new(name: &'a str, count: u32, sum: f32, min: f32, max: f32) -> Self {
        Self {
            name,
            count,
            sum,
            min,
            max,
        }
    }
}

impl<'a> PartialEq for StationResult<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

fn main() -> io::Result<()> {
    let f = File::open("/home/d/projects/1brc/src/main/java/measurements.txt")?;
    let size: usize = f.metadata()?.len() as usize;
    let fd = f.as_raw_fd();

    let ptr = unsafe {
        mmap64(
            std::ptr::null_mut(),
            size as libc::size_t,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            fd,
            0,
        )
    };

    if ptr == MAP_FAILED {
        return Err(io::Error::last_os_error());
    }

    let data = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };

    assert_eq!(data.len(), size);

    let chunks = chunker(size, data);
    let chunks_size = chunks.len();

    let mut handles = vec![];
    let (tx, rx) = channel();
    for chunk in chunks {
        let tx_clone: Sender<BTreeMap<&str, StationResult>> = tx.clone();
        let j_handle = thread::spawn(move || {
            chunk_processor(&data[chunk.0..chunk.1], tx_clone);
        });
        handles.push(j_handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let mut accumulated: BTreeMap<&str, StationResult> = BTreeMap::new();
    for _ in 0..chunks_size {
        let stations = rx.recv().unwrap();
        stations_accumulator(&mut accumulated, stations);
    }
    dbg!(accumulated.len());

    print!("{{");
    for (i, result) in accumulated.iter().enumerate() {
        if i == accumulated.len() - 1 {
            print!(
                "{}={:.1}/{}/{}",
                result.0,
                result.1.sum / result.1.count as f32, // mean
                result.1.min,
                result.1.max
            );
        } else {
            print!(
                "{}={:.1}/{}/{}, ",
                result.0,
                result.1.sum / result.1.count as f32, // mean
                result.1.min,
                result.1.max
            );
        }
    }
    print!("}}");
    println!();

    unsafe {
        if munmap(ptr, size) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

fn stations_accumulator<'a, 'b>(
    accumulated: &'b mut BTreeMap<&'a str, StationResult<'a>>,
    existing: BTreeMap<&'a str, StationResult<'a>>,
) {
    for station in existing {
        accumulated
            .entry(station.0)
            .and_modify(|e| {
                e.count += station.1.count;
                e.sum += station.1.sum;
                if e.max < station.1.max {
                    e.max = station.1.max;
                }
                if e.min > station.1.min {
                    e.min = station.1.min;
                }
            })
            .or_insert(station.1);
    }
}
