use crate::graph::*;
use std::fs::File;
use std::fs;
use std::io::{self, BufRead,BufWriter ,Write};
use std::path::Path;
use rustsat::instances::*;

// ファイルを読み込みグラフへ変換する関数
pub fn parse_graph_from_file(filename: &str) -> io::Result<Graph> {
    let path = Path::new(filename);
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);
    let mut g = Graph::new();
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.get(0) {
            Some(&"e") => {
                if parts.len() >= 3 {
                    if let (Ok(u), Ok(v)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                        g.add_edge(u, v);
                    }
                }
            }
            _ => (),
        }
    }
    Ok(g)
}

pub fn input_to_graph(filename: &str) -> Graph {
    match parse_graph_from_file(filename) {
        Ok(g) => g,
        Err(why) => panic!("couldn't open {}: {}", filename, why),
    }
}

//CNFファイルを書き出す関数
pub fn write_dimacs(cnf: Cnf, filename: &str) -> anyhow::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    let instance: SatInstance = SatInstance::from(cnf);
    instance.write_dimacs(&mut writer)
}

// ファイルに書き込む関数
pub fn _write_file(filename: &str, contents: &str) -> io::Result<()> {
    let mut file = File::create(filename)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn create_folder_if_not_exists(folder_name: &str) -> io::Result<()> {
    let path = Path::new(folder_name);
    if !path.exists() {
        fs::create_dir_all(path)?;
        // println!("Folder '{}' created.", folder_name);
    } else {
        // println!("Folder '{}' already exists.", folder_name);
    }
    Ok(())
}
