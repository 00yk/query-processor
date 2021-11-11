mod vbyte;
mod test;

use std::fs::File;
use std::collections::BTreeMap;
use inverted_list::*;
use mongodb::sync::Collection;
use std::io::{self, BufRead};
use std::io::Write;
use std::collections::VecDeque;
use aho_corasick::AhoCorasickBuilder;

use mongodb::{
    bson::doc,
    sync::Client,
};
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
static N: u32 = 3213835; //<- this is true value 3206234;<- this is estimated value.
static adl: f32 = 1128.2942; //1132.1028; <- this is estimated value
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
pub fn disjunctive_query(r: &mut File, lexicon: &BTreeMap<String, LexiconValue>, keywords: &Vec<String>, page_table: &BTreeMap<u32, (String, u32)>)
                    -> Vec<(u32, f32)>{

    let mut bunch_inverted_list: Vec<Vec<(u32, u32)>> = vec![];
    for word in keywords {
        if let Some(v) = lexicon.get(word) {
            let (term_ID, inverted_list) = read_inverted_list_from_offset(r, v.offset);
            bunch_inverted_list.push(inverted_list);
        }
    }
    // doc_ID to score
    let mut mp: BTreeMap<u32, f32> = BTreeMap::new();
    for inverted_list in bunch_inverted_list {
        // println!("inverted_list: {:?}", inverted_list);
        for (doc_ID, freq) in &inverted_list {
            // println!("doc_ID: {:?}", doc_ID);
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
pub fn conjunctive_query(r: &mut File, lexicon: &BTreeMap<String, LexiconValue>, keywords: &Vec<String>, page_table: &BTreeMap<u32, (String, u32)>)
                    -> Vec<(u32, f32)> {
    // returns a list of doc_IDs

    let mut bunch_inverted_list: Vec<Vec<(u32, u32)>> = vec![];
    for word in keywords {
        if let Some(v) = lexicon.get(word) {
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
            // println!("inverted_list: {:?}", inverted_list);
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
fn snippets_generation(docs: Vec<(u32, f32)>, coll: &Collection<Page>) -> String {
    // naively return first few lines for the doc
    let mut res = String::new();
    for (docID, freq) in docs {
        let cursor = coll.find(doc! { "docID": docID }, None).expect("MongoDB collection find failed");
        for result in cursor {
            // println!("content: {:?}", result.unwrap().content);
            let mut cnt = 0;
            for i in result.unwrap().content {
                // println!("{:?}", i);
                res.push_str(&format!("{}\n", i));
                cnt += 1;
                if cnt >= 3 {
                    break;
                }
            }
            // println!("------------------------------");
            res.push_str("---------------------------\n");
        }
    }
    res
}

fn solve(w: u8, content: &Vec<String>, patterns: &[String]) -> (usize, usize) {
    // returning the [start..end) slice of content
    if w == 0 {
        return (0, 0)
    }
    // assume w is nonnegative
    // if the window is larger than the whole context then just return all
    if content.len() <= w as usize {
        return (0, content.len())
    }

    let ac = AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .build(patterns);
    let mut window = VecDeque::new();
    // init window
    for i in 0..w {
        let cnt = ac.find_iter(&content[i as usize]).count();
        window.push_back(cnt);
    }
    let mut mx: usize = window.iter().sum();
    let mut cur = w;
    // slide window
    for i in w as usize..content.len() {
        window.pop_front();
        let cnt = ac.find_iter(&content[i as usize]).count();
        window.push_back(cnt);
        let sum = window.iter().sum();
        if sum > mx {
            mx = sum;
            cur = i as u8 + 1;
        }
    }

    (cur as usize - w as usize, cur as usize)
}
pub fn snippets_generation_ac(docs: Vec<(u32, f32)>, query: &Vec<String>, coll: &Collection<Page>) -> String {
    // based on Aho-corasick automaton
    let mut res = String::new();
    for (docID, freq) in docs {
        let cursor = coll.find(doc! { "docID": docID }, None).expect("MongoDB collection find failed");
        for result in cursor {
            let content = result.unwrap().content;
            // find the maximum patterns' occurrences in consecutive lines
            let (l, r) = solve(3, &content, query);
            for i in l..r {
                // println!("{:?}", content[i]);
                res.push_str(&format!("{}\n", content[i]));
            }

            // println!("------------------------------");
            res.push_str("---------------------------\n");
        }
    }
    res
}
