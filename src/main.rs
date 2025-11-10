use libc::{MAP_FAILED, mmap64, munmap};
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

fn find_next_semicolon(data: &[u8]) -> Vec<usize> {
    data.iter()
        .enumerate()
        .filter(|&(_, byte)| *byte == b';')
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

fn chunk_processor(data: &[u8], tx: Sender<StationResults>) {
    let mut station_results = StationResults {
        sum: 0.0,
        count: 0,
        min: ([0u8; 100], f32::INFINITY),
        max: ([0u8; 100], f32::NEG_INFINITY),
    };

    let mut min_max_buf = [0u8; 100];
    let mut station_begin = true;
    let mut start_float_cursor = 0usize;
    let mut start_line_cursor = 0usize;
    let mut semicolon_not_found = true;

    for (i, c) in data.iter().enumerate() {
        if station_begin && semicolon_not_found {
            min_max_buf[i - start_line_cursor] = *c;
            if data[i + 1] == b';' {
                semicolon_not_found = false;
            }
        } else if *c == b';' {
            station_begin = false;
            start_float_cursor = i + 1;
        } else if !station_begin && *c == b'\n' {
            let station_val = bytes_to_float(&data[start_float_cursor..i - 1])
                .expect("Should be a valid here...");
            station_results.sum += station_val;
            station_results.count += 1;
            if station_val > station_results.max.1 {
                let mut max0 = station_results.max.0;
                max0.clone_from_slice(&min_max_buf[..]);
                let min_max_buf_str = unsafe { str::from_utf8_unchecked(&min_max_buf[..]) };
                station_results.max.1 = station_val;
            } else if station_val < station_results.min.1 {
                let mut min0 = station_results.max.0;
                min0.clone_from_slice(&min_max_buf[..]);
                station_results.min.1 = station_val;
            }
            min_max_buf.fill(0);
            station_begin = true; // reset buffers
            start_float_cursor = 0;
            start_line_cursor = i + 1;
        }
    }

    tx.send(station_results).unwrap();
}

#[derive(Debug)]
struct StationResults {
    sum: f32,              // sum of vals
    count: u32,            // num stations
    min: ([u8; 100], f32), //name, val
    max: ([u8; 100], f32), //name, val
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
        let tx_clone: Sender<StationResults> = tx.clone();
        let j_handle = thread::spawn(move || {
            chunk_processor(&data[chunk.0..chunk.1], tx_clone);
        });
        handles.push(j_handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let mut total_sum = 0f32;
    let mut mean = 0f32;
    let mut total_count = 0;
    let mut max = 0f32;
    let mut min = 0f32;

    for _ in 0..chunks_size {
        let station = rx.recv().unwrap();
        total_sum += station.sum;
        total_count += station.count;
        if station.max.1 > max {
            max = station.max.1;
        }
        if station.min.1 > min {
            min = station.min.1;
        }
    }
    mean = total_sum / total_count as f32;

    println!("FINAL RESULT: max: {max}; min: {min}; mean: {mean}; sum: {total_sum}");

    unsafe {
        if munmap(ptr, size) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}
