use weird_client::WeirdClient;
use weird_core::world::Node;

const DELAY: std::time::Duration = std::time::Duration::from_millis(500);

fn main() {
    let weird = WeirdClient::connect().unwrap();

    let mut message = "Hello world!".to_string();

    loop {
        weird.render([
            Node::text(&message),
            Node::element("Button").id("run").child(Node::text("Run")),
            Node::element("Button").id("exit").child(Node::text("Exit")),
        ]);

        let Some(event) = weird.next_event().unwrap() else {
            break;
        };

        if event.is("run", "click") {
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
                        ]);
                        std::thread::sleep(DELAY);
                        weird.render([
                            Node::text("Running 5/5"),
                            Node::element("ProgressBar")
                                .attr("value", 5)
                                .attr("max", 5)
                                .children([Node::element("Other"), Node::text("Progress 5")]),
                        ]);
                        std::thread::sleep(DELAY);
                        message = "Finished running".to_string();
                    })
                    .join()
            })
            .unwrap();
        } else if event.is("exit", "click") {
            break;
        } else {
            message = "Unknown event".to_string();
        }
    }
}
