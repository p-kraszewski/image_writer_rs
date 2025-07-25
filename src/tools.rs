pub fn countdown(seconds: u64, dev: &str) {
    let msg = format!("Will start overwriting {dev} in");
    let bar = indicatif::ProgressBar::new(seconds)
        .with_message(msg)
        .with_position(seconds)
        .with_style(
            indicatif::ProgressStyle::with_template("{wide_bar} {msg} {pos}s, Press ^C to abort.")
                .unwrap(),
        );

    for n in 0 ..= seconds {
        bar.set_position(seconds - n);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    bar.finish_and_clear();
}
