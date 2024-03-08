use std::{collections::{hash_set, HashMap, HashSet}, fs::{self, File}, io::{self, BufRead}, path::{Path, PathBuf}};
use glob::glob;
use log::{error, trace, warn};
use ::petgraph::stable_graph::StableGraph;
use petgraph::{adj::EdgeIndex, data::Build, graph::{self, NodeIndex}, visit::GraphBase, Directed, Graph, Undirected};
use thiserror::Error;

pub mod petgraph {
    pub use petgraph::*;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("oh no! {0}")]
    OhNo(String),
    #[error("error msg: {0}")]
    ErrMsg(&'static str),
    #[error(transparent)]
    IO(#[from] std::io::Error)
}

pub fn get_tagged_files(root: &str) -> Result<HashSetGraph<TagGraphNode, Relation, Directed>, Error> {
    let pattern = format!("{}/**/*.tags", root);

    let mut tag_graph = HashSetGraph::<TagGraphNode, Relation, Directed>::new();

    let dir_root = tag_graph.get_node(&TagGraphNode::RootDirectory);
    let tag_root = tag_graph.get_node(&TagGraphNode::RootTag);

    trace!("Searching for tag files using {}", &pattern);
    for tagfile in glob(&pattern).expect("Failed to read glob pattern"){
        match tagfile {
            Ok(tagfile) => {
                trace!("Visiting tagfile {}", tagfile.as_path().to_string_lossy());
                let mut dirpath = tagfile.as_path().canonicalize()?;
                dirpath.pop();
                let dir = tag_graph.get_node_move(TagGraphNode::Directory { path: dirpath.clone() });
                match tagfile.file_name() {
                    Some(name) => {
                        // Collect the tag attach targets
                        let mut tag_attach_targets: Vec<NodeIndex> = vec![] ;
                        if name == "dir.tags" {
                            trace!("This is a directory tagfile. attach target: {:?}", dir);
                            tag_attach_targets.push(dir);
                        }
                        else {
                            // Files with the matching name
                            let tagfile_stem = tagfile.file_stem().unwrap();
                            let mut found = false;
                            for path in fs::read_dir(dirpath)?{
                                if let Ok(path) = path {
                                    let file_path = path.path();
                                    if let Some(ext) = file_path.extension(){
                                        // Don't associate a tagfile with itself
                                        if ext == "tags" {
                                            continue;
                                        }
                                    }
                                    let file_stem = file_path.file_stem().unwrap();
                                    let file_name = file_path.file_name().unwrap();
                                    if file_stem == tagfile_stem || file_name == tagfile_stem {
                                        found = true;
                                        trace!("Found file {}", file_path.to_string_lossy());
                                        let t = tag_graph.get_node_move(TagGraphNode::File { path: file_path });
                                        trace!("   ... assigned it {:?}", t);
                                        tag_attach_targets.push(t);
                                    }
                                }
                            }
                            if !found {
                                warn!("Tag file {:?} has no associated files", tagfile)
                            }
                        }

                        // Attach the tags to the targets
                        for tag in read_tagfile(&tagfile)? {
                            trace!("Tagfile contains tag {}", tag);
                            let t = tag_graph.get_node_move(TagGraphNode::Tag(tag.clone()));
                            tag_graph.graph.update_edge(tag_root, t, Relation::Tag);
                            tag_graph.graph.update_edge(tag_root, t, Relation::Tag);
                            for attach_target in &tag_attach_targets {
                                trace!("Attaching tag {:?} to {:?}", t, attach_target);
                                tag_graph.graph.update_edge(*attach_target, t, Relation::Tag);
                                tag_graph.graph.update_edge(t, *attach_target, Relation::Tag);
                            }
                        }
                    },
                    None => (),
                }
            },
            Err(_) => todo!(),
        }
    }
    Ok(tag_graph)
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
    pub graph: StableGraph<N, E, Ty>,
    map: HashMap<N, NodeIndex>
}

impl<N, E, Ty> HashSetGraph<N, E, Ty> 
where Ty: petgraph::EdgeType,
N: Eq + std::hash::Hash + Clone
{
    pub fn new() -> Self {
        Self {
            graph: StableGraph::default(),
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
pub enum TagGraphNode {
    File{ path: PathBuf },
    Directory { path: PathBuf },
    RootDirectory,
    RootTag,
    Tag(String)
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub enum Relation {
    Parent,
    Child,
    Tag,
}
