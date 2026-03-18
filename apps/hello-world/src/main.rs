use weird_client::WeirdClient;
use weird_core::world::NodeTree;

const DELAY: std::time::Duration = std::time::Duration::from_millis(500);

fn main() {
    let weird = WeirdClient::connect().unwrap();
    weird.render([NodeTree::text("Starting...")]);
    std::thread::sleep(DELAY);
    weird.render([
        NodeTree::text("Running 1/5"),
        NodeTree::element("ProgressBar")
            .attr("value", 1)
            .attr("max", 5)
            .children([NodeTree::element("Other"), NodeTree::text("Progress 1")]),
    ]);
    std::thread::sleep(DELAY);
    weird.render([
        NodeTree::text("Running 2/5"),
        NodeTree::element("ProgressBar")
            .attr("value", 2)
            .attr("max", 5)
            .children([NodeTree::element("Other"), NodeTree::text("Progress 2")]),
    ]);
    std::thread::sleep(DELAY);
    weird.render([
        NodeTree::text("Running 3/5"),
        NodeTree::element("ProgressBar")
            .attr("value", 3)
            .attr("max", 5)
            .children([NodeTree::element("Other"), NodeTree::text("Progress 3")]),
    ]);
    std::thread::sleep(DELAY);
    weird.render([
        NodeTree::text("Running 4/5"),
        NodeTree::element("ProgressBar")
            .attr("value", 4)
            .attr("max", 5)
            .children([NodeTree::element("Other"), NodeTree::text("Progress 4")]),
    ]);
    std::thread::sleep(DELAY);
    weird.render([
        NodeTree::text("Running 5/5"),
        NodeTree::element("ProgressBar")
            .attr("value", 5)
            .attr("max", 5)
            .children([NodeTree::element("Other"), NodeTree::text("Progress 5")]),
    ]);
    std::thread::sleep(DELAY);
    weird.render([NodeTree::text("Done")]);
}
