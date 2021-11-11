use std::fs::File;
use std::collections::BTreeMap;
use inverted_list::*;
use mongodb::sync::Collection;
use std::io::{self, BufRead};
use std::io::Write;
use std::collections::VecDeque;
use aho_corasick::AhoCorasickBuilder;


use query_processor::{conjunctive_query, disjunctive_query, snippets_generation_ac};

use mongodb::{
    bson::doc,
    sync::Client,
};

fn main() {
    let stdin = io::stdin();

    let mut f = File::open("inverted_index.tmp").unwrap();
    let lexicon: BTreeMap<String, LexiconValue> = deserialize_to_mem("lexicon.tmp").expect("Lexicon can not be read.");
    let page_table: BTreeMap<u32, (String, u32)> = deserialize_to_mem("page_table.tmp").expect("Page table cannot be read");

    // mongodb connection
    let client = Client::with_uri_str("mongodb://localhost:27017").expect("MongoDB connection failed");
    let database = client.database("wse");
    let collection = database.collection::<Page>("pages");


    let mut r = stdin.lock();
    loop {
        println!("Please input query type(q to exit): d(disjunctive), c(conjunctive): ");
        let mut typ = String::new();
        r.read_line(&mut typ).expect("Failed read");
        if typ == "q\n" {
            break;
        }
        while typ != "d\n" && typ != "c\n" {
            println!("Please input query type: d(disjunctive), c(conjunctive): ");
            typ.clear();
            r.read_line(&mut typ).expect("Failed read");
            if typ == "q\n" {
                break;
            }
        }
        println!("Please input keywords(enter to exit): ");
        let mut input = String::new();
        r.read_line(&mut input).expect("Failed read");
        let keywords: Vec<String> = input.split_whitespace().map(String::from).collect();
        if keywords.len() == 0 {
            break;
        }
        let docs = if typ == "d\n" {
            disjunctive_query(&mut f, &lexicon, &keywords, &page_table)
        } else if typ == "c\n" {
            conjunctive_query(&mut f, &lexicon, &keywords, &page_table)
        } else {
            vec![]
        };
        println!("docs {:?}", docs);
        println!("docs.len(): {:?}", docs.len());
        // snippets_generation(doc_IDs, &collection);
        let docs = docs.iter().map(|e| {(e.0, e.1)}).collect();
        let sni = snippets_generation_ac(docs, &keywords, &collection);
        println!("{}", sni);
    }
    println!("Exiting...");

}
