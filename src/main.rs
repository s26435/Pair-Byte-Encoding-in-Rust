use std::{collections::{HashMap, HashSet}, fs::File};
use std::io::{self, BufReader, BufRead, Write};
use indicatif::ProgressBar;
use serde::{Serialize, Deserialize};
use rayon::prelude::*;
use std::time::Instant;

//SVD, World2vec, Skipgram, CBOW - Doczytać

#[derive(Serialize, Deserialize)]
struct Tokenizer {
    training_data: Vec<String>,
    dict: Vec<String>,
}

impl Tokenizer {
    fn new(data: Vec<String>, dict: Vec<String>) -> Self {
        assert!(!dict.is_empty(), "starting dict can not be empty");
        assert!(!data.is_empty(), "data can not be empty");
        let training_data: Vec<String> = data.iter().flat_map(|line| {line.chars().map(|c| c.to_string())}).collect();
        println!("Len tokens: {}", training_data.len());
        return Self { training_data, dict };
    }

    fn train(&mut self, dict_size: usize) {
        assert!(!self.training_data.is_empty(), "you can't train on empty data set!");
        let pb = ProgressBar::new((dict_size-self.dict.len()) as u64);
        while self.dict.len() < dict_size {
            let start = Instant::now();
            let mut new_token_dict: HashMap<(String, String), i32> = HashMap::new();

            for i in 0..self.training_data.len().saturating_sub(1) {
                let pair = (
                    self.training_data[i].clone(),
                    self.training_data[i + 1].clone(),
                );
                *new_token_dict.entry(pair).or_insert(0) += 1;
            }

            let best_token = match new_token_dict.into_iter().max_by_key(|(_, v)| *v) {
                Some((k, _)) => k,
                None => break,
            };

            let merged = format!("{}{}", best_token.0, best_token.1).to_owned();
            self.dict.push(merged.clone());

            let new_vector = self.training_data.par_chunks(50_000).map(|chunk|{
                let mut out = Vec::with_capacity(chunk.len());
                let mut i = 0;
                while i < chunk.len(){
                    if i+1 < chunk.len() && &chunk[i] == &best_token.0 && &chunk[i+1] == &best_token.1{
                        out.push(merged.clone());
                        i+=2;
                    }else{
                        out.push(chunk[i].clone());
                        i+=1;
                    }
                }
                return out;
            }).reduce(||Vec::new(),|mut left, mut right|{
                if let (Some(l), Some(r)) = (left.last(), right.last()){
                    if l == &best_token.0 && r == &best_token.1{
                        left.pop();
                        right.remove(0);
                        left.push(merged.clone())
                    }
                }
                left.extend(right);
                left
            },);
            self.training_data = new_vector;
            let duration = start.elapsed();
            pb.inc(1);
        }
    }

    fn string_to_tokens(token_list: &Vec<String>, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let token_set: HashSet<&String> = token_list.iter().collect();
        let mut result: Vec<String> = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let mut matched: Option<(usize, String)> = None;

            for j in (i + 1..=chars.len()).rev() {
                let slice: String = chars[i..j].iter().collect();
                if token_set.contains(&slice) {
                    matched = Some((j, slice));
                    break;
                }
            }

            if let Some((next_i, token)) = matched {
                result.push(token);
                i = next_i;
            } else {
                panic!("undefined token at {}: {}", i, chars[i]);
            }
        }
        result
    }

    fn tokenize(&self, text: String) -> Vec<usize> {
        let tokens = Tokenizer::string_to_tokens(&self.dict, &text);
        tokens
            .iter()
            .map(|a| {
                self.dict.iter().position(|t| t == a).expect(&format!("token '{}' not found", a))
            })
            .collect()
    }

    fn save_to_file(&self, path: &str) {
        let json = serde_json::to_string_pretty(&self).expect("error during serialization");
        let mut file = File::create(path).expect("error during creating the file");
        file.write_all(json.as_bytes()).expect("write error");
    }

    fn load_from_file(path: &str) -> Self {
        let file = File::open(path).expect("error during opening file");
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).expect("deserialization error")
    }
}

fn get_seqs_from_fasta(path: &str, limit: usize) -> Vec<String> {
    let mut fasta = Vec::new();
    let file = File::open(path).expect("error during opening file");
    let reader = io::BufReader::new(file);
    let mut i:usize = 0;

    for line in reader.lines() {
        let con = line.expect("error while reading the line");
        if !con.starts_with('>') && !con.is_empty() {
            fasta.push(con);
            i+=1;
        }

        if i >= limit{
            break;
        }
    }
    return fasta;
}

fn train_new_and_predict(text: &String){
    let starting_dict: Vec<String> = vec![
        "A", "R", "N", "D", "C", "E", "Q", "G", "H", "I", "L", "K", "M", "F", "P", "S", "T", "W", "Y", "V",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    println!("Wczytywanie danych treningowych...");
    let train_data = get_seqs_from_fasta("seq.csv", 50000);
    println!("Długość danych treningowych: {:?}", train_data.len());
    println!("Rozpoczynanie trenowania...");

    let mut tokenizer = Tokenizer::new(train_data, starting_dict);
    tokenizer.train(10000);
    let x = tokenizer.tokenize(text.clone());
    println!("Tokeny: {:?}", x);
    println!("Skuteczność: {:?}", (x.len() as f64)/(text.clone().chars().as_str().len() as f64));
    tokenizer.save_to_file("tokenizer.json");
}

fn load_and_predict(text: &String){
    let tokenizer: Tokenizer = Tokenizer::load_from_file("tokenizer.json");
    let x = tokenizer.tokenize(text.clone());
    print!("{:?}", tokenizer.dict);
    println!("Tokeny: {:?}", x);
    println!("Skuteczność: {:?}", (x.len() as f64)/(text.clone().chars().as_str().len() as f64));
}

fn show_how_to_tokenize(text: &String){
    let tokenizer: Tokenizer = Tokenizer::load_from_file("tokenizer.json");
    let x = tokenizer.tokenize(text.clone());

    let tokens: Vec<String> = x.iter().map(|&a| tokenizer.dict[a].clone()).collect();
    println!("{:?}", text.clone());
    println!("{:?}", tokens);
}

fn main() {
    show_how_to_tokenize(&"CIRACKPDLSAETPMFPGNGDEQPLTENPRKYVM".to_string());
}