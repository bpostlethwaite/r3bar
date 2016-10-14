struct SysTime {}

impl SysTime {
    fn run<F>(send_string: F) where F: Fn(String) {
        thread::spawn(move || {

            let iv = Duration::from_millis(100);

            loop {
                let dt = Local::now();
                let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                send_string(time_str);

                thread::sleep(iv);
            }
        });
    }
}
