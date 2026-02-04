use std::{io::Write as _, os::unix::net::UnixStream, path::Path};

const DELAY: std::time::Duration = std::time::Duration::from_millis(500);

fn main() {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set");
    let mut weird = UnixStream::connect(Path::new(&runtime_dir).join("weird.sock"))
        .expect("weird.sock not found");

    writeln!(weird, "{}", r#"show ("Starting...")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(weird, "{}", r#"show ("Running 1/5")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(weird, "{}", r#"show ("Running 2/5")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(weird, "{}", r#"show ("Running 3/5")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(weird, "{}", r#"show ("Running 4/5")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(weird, "{}", r#"show ("Running 5/5")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        r#"show ("Done" @ProgressBar{total 1, extra 2, children (@Other{} "Text")})"#
    )
    .unwrap();
}
