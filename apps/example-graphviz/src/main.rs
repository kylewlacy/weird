use weird_client::WeirdClient;
use weird_core::world::Node;

fn main() {
    let weird = WeirdClient::builder()
        .app("example-graphviz")
        .connect()
        .unwrap();

    let mut engine = "dot".to_string();

    loop {
        weird.render([
            Node::text("Example graph"),
            Node::element("Graphviz")
                .id("graph")
                .attr(
                    "graph",
                    r#"
                        digraph {
                            a -> b
                        }
                    "#,
                )
                .attr("engine", engine.clone()),
            Node::text("Engine:"),
            Node::element("Select")
                .id("engine")
                .attr("value", engine.clone())
                .attr(
                    "choices",
                    [
                        "dot",
                        "neato",
                        "fdp",
                        "sfdp",
                        "circo",
                        "twopi",
                        "osage",
                        "patchwork",
                    ],
                ),
        ]);

        let Some(event) = weird.next_event().unwrap() else {
            break;
        };

        if event.is("engine", "change") {
            engine = event.param("value").unwrap();
        } else {
            // TODO: Log unknown event
        }
    }
}
