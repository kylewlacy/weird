use std::{io::Write as _, os::unix::net::UnixStream, path::Path};

fn main() {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set");
    let mut weird = UnixStream::connect(Path::new(&runtime_dir).join("weird.sock"))
        .expect("weird.sock not found");

    writeln!(
        weird,
        "{}",
        r#"show ( "Hello world!" @ProgressBar{total 1, extra 2, children (@Other{} "Text")} )"#
    )
    .unwrap();
}
