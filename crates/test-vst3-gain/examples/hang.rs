//! A module that never returns from loading.

kontra_vst3_gain::export_module!(
    2,
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
);
