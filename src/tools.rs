pub const BUF_SIZE: usize = 1024 * 1024;
pub const PAGE_SIZE: usize = 4096;
pub fn countdown(seconds: u64, dev: &str) {
    let msg = format!("Will start overwriting {dev} in");
    let bar = indicatif::ProgressBar::new(seconds)
        .with_message(msg)
        .with_position(seconds)
        .with_style(
            indicatif::ProgressStyle::with_template("{wide_bar} {msg} {pos}s, Press ^C to abort.")
                .unwrap(),
        );

    for n in 0..=seconds {
        bar.set_position(seconds - n);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    bar.finish_and_clear();
}

pub fn eyre_unroll(e: color_eyre::Report) -> String {
    e.chain()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(": ")
}

pub struct AlignedBuffer {
    buf:        Box<[u8; BUF_SIZE + PAGE_SIZE]>,
    page_shift: usize,
    pub used:   usize,
}

impl AlignedBuffer {
    pub fn new() -> AlignedBuffer {
        let buf = Box::new([0u8; BUF_SIZE + PAGE_SIZE]);
        let page_shift = (PAGE_SIZE - ((buf.as_ptr() as usize) & (PAGE_SIZE - 1))) % PAGE_SIZE;
        let used = 0;

        Self {
            buf,
            page_shift,
            used,
        }
    }

    pub fn get_aligned_buf(&mut self) -> &mut [u8] {
        &mut self.buf[self.page_shift..self.page_shift + BUF_SIZE]
    }
}
