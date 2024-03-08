use ::petgraph::stable_graph::StableGraph;
use glob::glob;
use log::{error, trace, warn};
use petgraph::{
    adj::EdgeIndex,
    data::Build,
    graph::{self, NodeIndex},
    visit::GraphBase,
    Directed, Graph, Undirected,
};
use std::{
    collections::{hash_set, HashMap, HashSet},
    fs::{self, File, FileType},
    io::{self, BufRead},
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

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
    IO(#[from] std::io::Error),
}

pub fn get_tagged_files(
    root: &str,
) -> Result<HashSetGraph<TagGraphNode, Relation, Directed>, Error> {
    let mut tag_graph = HashSetGraph::<TagGraphNode, Relation, Directed>::new();

    add_tags_to_graph(root, &mut tag_graph)?;
    add_file_structure_to_graph(root, &mut tag_graph)?;

    Ok(tag_graph)
}

fn add_tags_to_graph(
    root: &str,
    tag_graph: &mut HashSetGraph<TagGraphNode, Relation, Directed>,
) -> Result<(), Error> {
    let tag_root = tag_graph.get_node(&TagGraphNode::RootTag);
    let pattern = format!("{}/**/*.tags", root);
    trace!("Searching for tag files using {}", &pattern);
    for tagfile in glob(&pattern).expect("Failed to read glob pattern") {
        match tagfile {
            Ok(tagfile) => {
                trace!("Visiting tagfile {}", tagfile.as_path().to_string_lossy());
                let mut dirpath = tagfile.as_path().canonicalize()?;
                dirpath.pop();
                let dir = tag_graph.get_node_move(TagGraphNode::Directory {
                    path: dirpath.clone(),
                });
                match tagfile.file_name() {
                    Some(name) => {
                        // Collect the tag attach targets
                        let mut tag_attach_targets: Vec<NodeIndex> = vec![];
                        if name == "dir.tags" {
                            trace!("This is a directory tagfile. attach target: {:?}", dir);
                            tag_attach_targets.push(dir);
                        } else {
                            // Files with the matching name
                            let tagfile_stem = tagfile.file_stem().unwrap();
                            let mut found = false;
                            for path in fs::read_dir(dirpath)? {
                                if let Ok(path) = path {
                                    let file_path = path.path();
                                    if let Some(ext) = file_path.extension() {
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
                                        let t = tag_graph
                                            .get_node_move(TagGraphNode::File { path: file_path });
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
                            tag_graph.graph.update_edge(tag_root, t, Relation::HasTag);
                            tag_graph.graph.update_edge(tag_root, t, Relation::HasTag);
                            for attach_target in &tag_attach_targets {
                                trace!("Attaching tag {:?} to {:?}", t, attach_target);
                                tag_graph
                                    .graph
                                    .update_edge(*attach_target, t, Relation::HasTag);
                                tag_graph
                                    .graph
                                    .update_edge(t, *attach_target, Relation::TagAssignedTo);
                            }
                        }
                    }
                    None => (),
                }
            }
            Err(_) => todo!(),
        }
    }
    Ok(())
}

fn add_file_structure_to_graph(
    root: &str,
    tag_graph: &mut HashSetGraph<TagGraphNode, Relation, Directed>,
) -> Result<(), Error> {
    let dir_root = tag_graph.get_node(&TagGraphNode::RootDirectory);
    for entry in WalkDir::new(root) {
        match entry {
            Ok(entry) => {
                let path = entry.path().canonicalize().unwrap();
                if let Some(extension) = path.extension() {
                    if extension == "tags" {
                        continue;
                    }
                }

                let node = if path.is_dir() {
                    tag_graph.get_node_move(TagGraphNode::Directory {
                        path: path.to_path_buf(),
                    })
                } else {
                    tag_graph.get_node_move(TagGraphNode::File {
                        path: path.to_path_buf(),
                    })
                };

                if entry.depth() == 0 {
                    tag_graph.graph.update_edge(dir_root, node, Relation::Child);
                    tag_graph
                        .graph
                        .update_edge(node, dir_root, Relation::Parent);
                } else {
                    let parent = tag_graph.get_node_move(TagGraphNode::Directory {
                        path: path.parent().unwrap().canonicalize().unwrap().to_path_buf(),
                    });
                    tag_graph.graph.update_edge(parent, node, Relation::Child);
                    tag_graph.graph.update_edge(node, parent, Relation::Parent);
                }
            }
            Err(e) => {
                error!("Error when walking file structure: {:?}", e);
            }
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
where
    Ty: petgraph::EdgeType,
    N: Eq + std::hash::Hash + Clone,
{
    pub graph: StableGraph<N, E, Ty>,
    map: HashMap<N, NodeIndex>,
}

impl<N, E, Ty> HashSetGraph<N, E, Ty>
where
    Ty: petgraph::EdgeType,
    N: Eq + std::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            graph: StableGraph::default(),
            map: HashMap::new(),
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
    File { path: PathBuf },
    Directory { path: PathBuf },
    RootDirectory,
    RootTag,
    Tag(String),
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub enum Relation {
    // Directory/File A's parent is B
    Parent,
    // Directory B contains A
    Child,
    // Directory/File A has tag B
    HasTag,
    // Tag A has been assigned to B
    TagAssignedTo,
}
