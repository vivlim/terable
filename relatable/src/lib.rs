use std::{collections::{hash_set, HashMap, HashSet}, fs::File, io::{self, BufRead}, path::{Path, PathBuf}};
use glob::glob;
use petgraph::{adj::EdgeIndex, data::Build, graph::{self, NodeIndex}, visit::GraphBase, Directed, Graph, Undirected};
use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("oh no! {0}")]
    OhNo(String),
    #[error(transparent)]
    IO(#[from] std::io::Error)
}

pub fn get_tagged_files(root: &str) -> Result<(), Error> {
    let pattern = format!("{}/**/*.tags", root);
    println!("Pattern: {}", pattern);

    let mut tag_graph = HashSetGraph::<TagGraphNode, Relation, Directed>::new();

    let dir_root = tag_graph.get_node(&TagGraphNode::RootDirectory);
    let tag_root = tag_graph.get_node(&TagGraphNode::RootTag);

    for tagfile in glob(&pattern).expect("Failed to read glob pattern"){
        match tagfile {
            Ok(tagfile) => {
                let mut dirpath = tagfile.as_path().canonicalize()?;
                dirpath.pop();
                let dir = tag_graph.get_node_move(TagGraphNode::Directory { path: dirpath.clone() });
                match tagfile.file_name() {
                    Some(name) => {
                        // Collect the tag attach targets
                        let mut tag_attach_targets: Vec<NodeIndex> = vec![] ;
                        if name == "dir.tags" {
                            tag_attach_targets.push(dir);
                        }
                        else {
                            // Files with the matching name
                            let name_without_tags_suffix = tagfile.file_stem().unwrap();
                            let pattern = format!("{}*", dirpath.join(name_without_tags_suffix).to_string_lossy());
                            for target_file in glob(&pattern).expect("Failed to read glob pattern") {
                                match target_file {
                                    Ok(target_file) => {
                                        let target_file_path = target_file.as_path().canonicalize()?;
                                        let t = tag_graph.get_node_move(TagGraphNode::File { path: target_file_path });

                                    },
                                    Err(_) => todo!()
                                }


                            }
                            
                            
                            
                        }

                        // Attach the tags to the targets
                        for tag in read_tagfile(&tagfile)? {
                            let t = tag_graph.get_node_move(TagGraphNode::Tag(tag));
                            tag_graph.graph.update_edge(tag_root, t, Relation::Tag);
                            for attach_target in &tag_attach_targets {
                                tag_graph.graph.update_edge(*attach_target, t, Relation::Tag);
                            }
                        }

                    },
                    None => (),
                }
            },
            Err(_) => todo!(),
        }
    }
    Ok(())
}

/// Reads a tag file
/// A tag file is simply a text file where each line is a tag
pub fn read_tagfile(file: &PathBuf) -> Result<Vec<String>, Error> {
    let file = File::open(file)?;
    let mut tags = vec![];
    for line in io::BufReader::new(file).lines() {
        tags.push(line?);
    }
    Ok(tags)
}

pub struct HashSetGraph<N, E, Ty>
where Ty: petgraph::EdgeType,
N: Eq + std::hash::Hash + Clone
{
    pub graph: Graph<N, E, Ty>,
    map: HashMap<N, NodeIndex>
}

impl<N, E, Ty> HashSetGraph<N, E, Ty> 
where Ty: petgraph::EdgeType,
N: Eq + std::hash::Hash + Clone
{
    pub fn new() -> Self {
        Self {
            graph: Graph::default(),
            map: HashMap::new()
        }
    }

    /// Gets the index of a node. Adds it to the graph if it didn't already exist.
    pub fn get_node(&mut self, weight: &N) -> NodeIndex {
        if let Some(existing) = self.map.get(weight) {
            return *existing;
        }

        let idx = self.graph.add_node(weight.clone());
        self.map.insert(weight.clone(), idx);
        idx
    }

    /// Gets the index of a node. Adds it to the graph if it didn't already exist.
    pub fn get_node_move(&mut self, weight: N) -> NodeIndex {
        if let Some(existing) = self.map.get(&weight) {
            return *existing;
        }

        let idx = self.graph.add_node(weight.clone());
        self.map.insert(weight.clone(), idx);
        idx
    }

    /// Updates an edge between two nodes. The nodes are created if they didn't exist.
    pub fn update_edge(&mut self, a: &N, b: &N, weight: E) {
        let ax = self.get_node(&a);
        let bx = self.get_node(&b);
        self.graph.update_edge(ax, bx, weight);
    }
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
enum TagGraphNode {
    File{ path: PathBuf },
    Directory { path: PathBuf },
    RootDirectory,
    RootTag,
    Tag(String)
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
enum Relation {
    Parent,
    Child,
    Tag,
}
