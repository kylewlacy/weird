use weird_client::WeirdClient;
use weird_core::world::Node;

const DELAY: std::time::Duration = std::time::Duration::from_millis(500);

fn main() {
    let weird = WeirdClient::builder().connect().unwrap();

    let mut name = "".to_string();
    let mut message: Option<String> = None;

    loop {
        let current_message;
        let current_message = if let Some(message) = &message {
            message
        } else if name.is_empty() {
            "Hello world!"
        } else {
            current_message = format!("Hello, {name}!");
            &current_message
        };
        weird.render([
            Node::text(current_message),
            Node::element("Input")
                .id("name")
                .attr("value", &name)
                .attr("placeholder", "Your name"),
            Node::element("Button").id("run").child(Node::text("Run")),
            Node::element("Button").id("exit").child(Node::text("Exit")),
        ]);

        let Some(event) = weird.next_event().unwrap() else {
            break;
        };

        if event.is("name", "change") {
            name = event.param("value").unwrap();
            name = name.trim().to_string()
        } else if event.is("run", "click") {
            std::thread::scope(|scope| {
                scope
                    .spawn(|| {
                        weird.render([
                            Node::text("Running 1/5"),
                            Node::element("ProgressBar")
                                .attr("value", 1)
                                .attr("max", 5)
                                .children([Node::element("Other"), Node::text("Progress 1")]),
                        ]);
                        std::thread::sleep(DELAY);
                        weird.render([
                            Node::text("Running 2/5"),
                            Node::element("ProgressBar")
                                .attr("value", 2)
                                .attr("max", 5)
                                .children([Node::element("Other"), Node::text("Progress 2")]),
                        ]);
                        std::thread::sleep(DELAY);
                        weird.render([
                            Node::text("Running 3/5"),
                            Node::element("ProgressBar")
                                .attr("value", 3)
                                .attr("max", 5)
                                .children([Node::element("Other"), Node::text("Progress 3")]),
                        ]);
                        std::thread::sleep(DELAY);
                        weird.render([
                            Node::text("Running 4/5"),
                            Node::element("ProgressBar")
                                .attr("value", 4)
                                .attr("max", 5)
                                .children([Node::element("Other"), Node::text("Progress 4")]),
                            Node::element("Box")
                                .attr("id", "label1")
                                .child(Node::text("almost done...")),
                            Node::element("Box")
                                .attr("id", "label2")
                                .child(Node::text("...")),
                        ]);
                        std::thread::sleep(DELAY);
                        weird.render([
                            Node::text("Running 5/5"),
                            Node::element("ProgressBar")
                                .attr("value", 5)
                                .attr("max", 5)
                                .children([Node::element("Other"), Node::text("Progress 5")]),
                            Node::element("Box")
                                .attr("id", "label2")
                                .child(Node::text("almost done...")),
                            Node::element("Box")
                                .attr("id", "label3")
                                .child(Node::text("...")),
                            Node::element("Box")
                                .attr("id", "label1")
                                .child(Node::text("...")),
                        ]);
                        std::thread::sleep(DELAY * 3);
                        message = Some("Finished running".to_string());
                    })
                    .join()
            })
            .unwrap();
        } else if event.is("exit", "click") {
            break;
        } else {
            message = Some("Unknown event".to_string());
        }
    }
}
