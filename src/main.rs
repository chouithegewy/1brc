use libc::{MAP_FAILED, mmap64, munmap};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::thread;

fn bytes_to_float(bytes: [u8; 5]) -> Option<f32> {
    //naive first
    let sign = if bytes[0] == 45 {
        // minus sign (-)
        1
    } else {
        0
    };
    let mut integer_part = -100;
    let mut fractional_part = -0.1f32;
    let mut point: usize = 0;
    for i in sign..bytes.len() {
        if bytes[i] == b'.' {
            point = i;
        }
    }
    fractional_part = match point {
        1 => 1f32 / bytes[2] as f32,
        2 => 1f32 / bytes[3] as f32,
        3 => 1f32 / bytes[4] as f32,
        _ => panic!("Failed to parse float -- invalid index"),
    };
    integer_part = match point {
        1 => bytes[0] as i32,
        2 => bytes[0] as i32 * 10 + bytes[1] as i32,
        3 => bytes[1] as i32 * 10 + bytes[2] as i32,
        _ => panic!("Failed to parse float -- invalid index"),
    };

    Some(integer_part as f32 + fractional_part)

    //let offset: usize = if sign > 0 { 0 } else { 1 };

    //let integer_part = (bytes[offset + 1] as u16) | ((bytes[0] as u16) << 8);

    //if integer_part > 99 {
    //    return None;
    //}

    //let fractional_part = bytes[2] as f32 / 10.0;
    //Some(integer_part as f32 + fractional_part)
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


fn get_thread_count() -> usize {
    thread::available_parallelism()
        .expect("Failed to get available_parallelism")
        .into()
}

// provide chunks on data for parallel processing that include full lines
fn chunker(size: usize, data: &[u8]) -> Vec<(usize, usize)> {
    let thread_count = get_thread_count();
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

fn chunk_processor(data: &[u8]) {
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
    dbg!(&chunks);

    let mut handles = vec![];
    for chunk in chunks {
        let j_handle = thread::spawn(move || println!("process ze data: chunk {:?}", chunk));
        handles.push(j_handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    unsafe {
        if munmap(ptr, size) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}
