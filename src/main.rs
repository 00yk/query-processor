use std::fs::File;
use std::collections::BTreeMap;
use inverted_list::*;
use std::io::{self, BufRead};
use std::io::Write;
fn nextGEQ(doc_ID: u32, inverted_list: &Vec<(u32, u32)>, mut cur_pos: usize) -> (u32, usize) {
    // next greater than or equal to doc_ID
    // linear search from cur_pos
    let length = inverted_list.len();
    while cur_pos < length && inverted_list[cur_pos].0 < doc_ID {
        cur_pos += 1;
    }
    // return doc_ID
    if cur_pos < length {
        return (inverted_list[cur_pos].0, cur_pos);
    }
    (u32::MAX, cur_pos)
}
static k: f32 = 1.2;
static b: f32 = 0.75;
static N: u32 = 3206234; // this is estimate, use true value later!
static adl: f32 = 1132.1028;
fn BM25(term_freq: u32, nt: u32, dl: u32) -> f32 {
    // Reference:
    // https://kmwllc.com/index.php/2020/03/20/understanding-tf-idf-and-bm-25/
    //
    // N is the number of docs in corpus, adl is average doc length for corpus
    // nt is the length of inverted_list for term i
    // k and b are hyperparameters
    // dl is the document lenght
    let idf = ((N as f32 - nt as f32 + 0.5) / (nt as f32 + 0.5)).log2();
    let tf = term_freq as f32 / (term_freq as f32 + k * ((1.0 - b) + b * dl as f32 / adl));
    idf * tf
}
fn zigzag_join(bunch_inverted_list: Vec<Vec<(u32, u32)>>, maxDocID: u32, page_table: &BTreeMap<u32, (String, u32)>) -> Vec<(u32, f32)> {
    let mut did = 0; // doc_ID
    let n = bunch_inverted_list.len();
    let mut curs = vec![0; n];
    let mut res: Vec<(u32, f32)> = vec![];
    'outer: while did < maxDocID {
        let ret = nextGEQ(did, &bunch_inverted_list[0], curs[0]);
        did = ret.0;
        let pos = ret.1;
        curs[0] = pos;
        let mut d = 0;
        for i in 1..n {
            let ret = nextGEQ(did, &bunch_inverted_list[i], curs[i]);
            d = ret.0;
            if d == u32::MAX {
                // one inverted_list reach end
                break 'outer;
            }
            let pos = ret.1;
            curs[i] = pos;
        }
        if d > did {
            did = d;
        } else {
            // doc_ID is in intersection
            let mut score: f32 = 0.0;
            for i in 0..n {
                let inverted_list = &bunch_inverted_list[i];  //inverted_list for term i
                let freq = inverted_list[curs[i]].1;
                let dl = page_table.get(&did).unwrap().1;
                score += BM25(freq, inverted_list.len() as u32, dl); // did is the current doc_ID that contains all words
            }
            res.push((did, score));
            did += 1;
        }

    }
    res
}
fn disjunctive_query(r: &mut File, lexicon: &BTreeMap<String, LexiconValue>, keywords: Vec<String>, page_table: &BTreeMap<u32, (String, u32)>)
                    -> Vec<(u32, f32)>{

    let mut bunch_inverted_list: Vec<Vec<(u32, u32)>> = vec![];
    for word in keywords {
        if let Some(v) = lexicon.get(&word) {
            let (term_ID, inverted_list) = read_inverted_list_from_offset(r, v.offset);
            bunch_inverted_list.push(inverted_list);
        }
    }
    // doc_ID to score
    let mut mp: BTreeMap<u32, f32> = BTreeMap::new();
    for inverted_list in bunch_inverted_list {
        for (doc_ID, freq) in &inverted_list {
            let doc_length = page_table.get(&doc_ID).unwrap().1;
            let score_new = BM25(*freq, inverted_list.len() as u32, doc_length);
            if let Some(score) = mp.get_mut(&doc_ID) {
                *score += score_new;

            } else {
                mp.insert(*doc_ID, score_new);
            }
        }
    }
    let mut res = vec![];
    for (did, score) in mp {
        res.push((did, score));
    }
    res.sort_by(|p, q| q.1.partial_cmp(&p.1).expect("Not expecting a NaN to appear in BM25 score"));
    if res.len() > 10 {
        return res[0..10].to_vec();
    }
    res
}
fn conjunctive_query(r: &mut File, lexicon: &BTreeMap<String, LexiconValue>, keywords: Vec<String>, page_table: &BTreeMap<u32, (String, u32)>)
                    -> Vec<(u32, f32)> {
    // returns a list of doc_IDs

    let mut bunch_inverted_list: Vec<Vec<(u32, u32)>> = vec![];
    for word in keywords {
        if let Some(v) = lexicon.get(&word) {
            let (term_ID, inverted_list) = read_inverted_list_from_offset(r, v.offset);
            bunch_inverted_list.push(inverted_list);
        } else {
            return vec![];
        }

    }
    // if length of bunch_inverted_list is 0 or 1, then don't call zigzag_join
    let mut res = if bunch_inverted_list.len() <= 1{
        let mut res: Vec<(u32, f32)> = vec![];
        for inverted_list in bunch_inverted_list {
            for i in &inverted_list {
                let mut score = 0.0;
                let doc_length = page_table.get(&i.0).unwrap().1;
                score += BM25(i.1, inverted_list.len() as u32, doc_length);
                res.push((i.0, score));
            }
        }
        res
    } else {
        bunch_inverted_list.sort_unstable_by_key(|v| v.len());
        let res = zigzag_join(bunch_inverted_list, *page_table.iter().next_back().unwrap().0, page_table);
        res
    };
    res.sort_by(|p, q| q.1.partial_cmp(&p.1).expect("Not expecting there's document's BM25 score is NaN."));
    // res.sort_by(|arg1, arg2| arg1.1.partial_cmp(&arg2.1).unwrap_or(std::cmp::Ordering::Less));
    if res.len() > 10{
        return res[..10].to_vec();
    }
    res
}
fn main() {
    let stdin = io::stdin();

    let mut f = File::open("inverted_index.tmp").unwrap();
    let lexicon: BTreeMap<String, LexiconValue> = deserialize_to_mem("lexicon.tmp").expect("Lexicon can not be read.");
    let page_table: BTreeMap<u32, (String, u32)> = deserialize_to_mem("page_table.tmp").expect("Page table cannot be read");


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
        let doc_IDs = if typ == "d\n" {
            disjunctive_query(&mut f, &lexicon, keywords, &page_table)
        } else if typ == "c\n" {
            conjunctive_query(&mut f, &lexicon, keywords, &page_table)
        } else {
            vec![]
        };
        println!("doc_ID {:?}", doc_IDs);
    }
    println!("Exiting...");

}
