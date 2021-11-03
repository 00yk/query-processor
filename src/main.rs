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
fn zigzag_join(bunch_inverted_list: Vec<Vec<(u32, u32)>>, maxDocID: u32) -> Vec<u32> {
    println!("maxDocID: {:?}", maxDocID);
    let mut did = 0; // doc_ID
    let n = bunch_inverted_list.len();
    let mut curs = vec![0; n];
    let mut res = vec![];
    while did < maxDocID {
        let ret = nextGEQ(did, &bunch_inverted_list[0], curs[0]);
        println!("{:?}", ret);
        did = ret.0;
        let pos = ret.1;
        curs[0] = pos;
        let mut d = 0;
        for i in 1..n {
            let ret = nextGEQ(did, &bunch_inverted_list[i], curs[i]);
            d = ret.0;
            let pos = ret.1;
            curs[i] = pos;
        }
        if d > did {
            did = d;
        } else {
            // doc_ID is in intersection
            // let freqs = vec![];
            // for i in 0..n {
            //     freqs.push(&bunch_inverted_list[i][did as usize].1);
            // }
            // TODO: compute BM25
            // fake score
            // let score: u32 = freqs.iter().sum();
            res.push(did);
            did += 1;
        }

    }
    res
}
fn conjuctive_query(r: &mut File, lexicon: &BTreeMap<String, LexiconValue>, keywords: Vec<String>, page_table: &BTreeMap<u32, String>) -> Vec<u32> {
    // returns a list of doc_IDs

    let mut bunch_inverted_list: Vec<Vec<(u32, u32)>> = vec![];
    for word in keywords {
        if let Some(v) = lexicon.get(&word) {
            let (term_ID, inverted_list) = read_inverted_list_from_offset(r, v.offset);
            println!("{:?}", inverted_list);
            bunch_inverted_list.push(inverted_list);
        } else {
            return vec![];
        }

    }
    // if length of bunch_inverted_list is 0 or 1, then don't call zigzag_join
    if bunch_inverted_list.len() <= 1{
        let mut res = vec![];
        for inverted_list in bunch_inverted_list {
            for i in inverted_list {
                res.push(i.0);
            }
        }
        return res;
    }
    bunch_inverted_list.sort_unstable_by_key(|v| v.len());
    let res = zigzag_join(bunch_inverted_list, *page_table.iter().next_back().unwrap().0);

    res
}
fn main() {
    let stdin = io::stdin();

    let mut f = File::open("inverted_index.tmp").unwrap();
    let lexicon: BTreeMap<String, LexiconValue> = deserialize_to_mem("lexicon.tmp").unwrap();
    let page_table: BTreeMap<u32, String> = deserialize_to_mem("page_table.tmp").unwrap();


    let mut r = stdin.lock();
    while true {
        println!("Please input number of words, and then following n words: ");
        let mut input = String::new();
        r.read_line(&mut input).expect("Failed read");
        let keywords: Vec<String> = input.split_whitespace().map(String::from).collect();

        let doc_IDs = conjuctive_query(&mut f, &lexicon, keywords, &page_table);
        println!("doc_ID {:?}", doc_IDs);
    }

    // for i in 0..n {
    //     let v = lexicon.get(&keywords[i]).unwrap();
    //     let (term_ID, inverted_list) = read_inverted_list_from_offset(&mut f, v.offset);
    //     println!("{:?} {:?} {:?}", keywords[i], term_ID, inverted_list);
    // }

    // let doc_IDs = conjuctive_query(f, lexicon, keywords, page_table);
    // println!("doc_ID {:?}", doc_IDs);

}
