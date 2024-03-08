use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, Graph, GraphView, SettingsInteraction, SettingsNavigation,
    SettingsStyle,
};
use relatable::{
    petgraph::{csr::DefaultIx, data::DataMap, visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences, NodeRef}, Directed},
    HashSetGraph, Relation, TagGraphNode,
};

pub struct TemplateApp {
    graph: Graph<TagGraphNode, Relation, Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>,
    relatable_graph: HashSetGraph<TagGraphNode, Relation, Directed>,
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let relatable_graph = relatable::get_tagged_files("s:/git/terable/testdata/").unwrap();
        let mut graph: Graph<TagGraphNode, Relation, Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape> = (&relatable_graph.graph).into();

        for (index, weight) in relatable_graph.graph.node_references() {
            graph.node_mut(index).unwrap().set_label(match weight{
                TagGraphNode::File { path } => path.file_name().expect("a file node should have a filename").to_string_lossy().to_string(),
                TagGraphNode::Directory { path } => format!("{}/", path.file_name().expect("a directory node should have a name").to_string_lossy()),
                TagGraphNode::RootDirectory => "ROOT_DIR".to_string(),
                TagGraphNode::RootTag => "ROOT_TAG".to_string(),
                TagGraphNode::Tag(t) => format!("[{}]", t),
            });
        }

        for e in relatable_graph.graph.edge_references() {
            graph.edge_mut(e.id()).unwrap().set_label(format!("{:?}", e.weight()));
        }


        TemplateApp {
            graph: graph,
            relatable_graph,
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            for node in self.graph.selected_nodes() {
                ui.label(format!("node {:?}", node.id()));
                let data = self.relatable_graph.graph.node_weight(*node);
                ui.label(format!("node {}", node.index()));
            }
            // for edge in self.graph.selected_edges() {
            //     ui.label(format!("edge {}: {:?}", edge.index(), edge.()));
            // }

            
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                &mut GraphView::<_, _, _, _, DefaultNodeShape, DefaultEdgeShape>::new(
                    &mut self.graph,
                )
                .with_navigations(&SettingsNavigation::default().with_zoom_and_pan_enabled(true))
                .with_interactions(
                    &SettingsInteraction::default()
                        .with_node_selection_enabled(true)
                        .with_edge_selection_enabled(true)
                        .with_dragging_enabled(true)
                        .with_node_clicking_enabled(true),
                )
                .with_styles(&SettingsStyle::default().with_labels_always(true)),
            );
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
