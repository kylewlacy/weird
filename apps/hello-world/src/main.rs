use std::{io::Write as _, os::unix::net::UnixStream, path::Path};

const DELAY: std::time::Duration = std::time::Duration::from_millis(500);

fn main() {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set");
    let mut weird = UnixStream::connect(Path::new(&runtime_dir).join("weird.sock"))
        .expect("weird.sock not found");

    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "render",
            "params": ["Starting..."],
        })
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "2",
            "method": "render",
            "params": [
                "Running 1/5",
                {
                    "tag": "ProgressBar",
                    "attributes": {
                        "value": 1,
                        "max": 5,
                    },
                    "children": [
                        {"tag": "Other"},
                        "Progress 1"
                    ]
                }
            ],
        })
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "3",
            "method": "render",
            "params": [
                "Running 2/5",
                {
                    "tag": "ProgressBar",
                    "attributes": {
                        "value": 2,
                        "max": 5,
                    },
                    "children": [
                        {"tag": "Other"},
                        "Progress 2"
                    ]
                }
            ],
        })
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "4",
            "method": "render",
            "params": [
                "Running 3/5",
                {
                    "tag": "ProgressBar",
                    "attributes": {
                        "value": 3,
                        "max": 5,
                    },
                    "children": [
                        {"tag": "Other"},
                        "Progress 3"
                    ]
                }
            ],
        })
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "5",
            "method": "render",
            "params": [
                "Running 4/5",
                {
                    "tag": "ProgressBar",
                    "attributes": {
                        "value": 4,
                        "max": 5,
                    },
                    "children": [
                        {"tag": "Other"},
                        "Progress 4"
                    ]
                }
            ],
        })
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "6",
            "method": "render",
            "params": [
                "Running 5/5",
                {
                    "tag": "ProgressBar",
                    "attributes": {
                        "value": 5,
                        "max": 5,
                    },
                    "children": [
                        {"tag": "Other"},
                        "Progress 5"
                    ]
                }
            ],
        })
    )
    .unwrap();
    std::thread::sleep(DELAY);
    writeln!(
        weird,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "2",
            "method": "render",
            "params": [
                "Done"
            ],
        })
    )
    .unwrap();
}
