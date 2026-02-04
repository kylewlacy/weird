use std::{io::Write as _, os::unix::net::UnixStream, path::Path};

const DELAY: std::time::Duration = std::time::Duration::from_millis(500);

fn main() {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set");
    let mut weird = UnixStream::connect(Path::new(&runtime_dir).join("weird.sock"))
        .expect("weird.sock not found");

    writeln!(weird, "{}", r#"render ("Starting...")"#).unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        r#"render ("Running 1/5" @ProgressBar{value 1, max 5, children (@Other{} "Progress 1")})"#
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        r#"render ("Running 2/5" @ProgressBar{value 2, max 5, children (@Other{} "Progress 2")})"#
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        r#"render ("Running 3/5" @ProgressBar{value 3, max 5, children (@Other{} "Progress 3")})"#
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        r#"render ("Running 4/5" @ProgressBar{value 4, max 5, children (@Other{} "Progress 4")})"#
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        r#"render ("Running 5/5" @ProgressBar{value 5, max 5, children (@Other{} "Progress 5")})"#
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(weird, "{}", r#"render ("Done")"#).unwrap();
}
