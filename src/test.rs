#[cfg(test)]
#[test]
fn test_vbyte() {
    use crate::vbyte::*;
    let a = vbyteEncodeNumber(1024);
    println!("{:?}", a);

    let v = vec![1, 2, 3, 1024];
    let b = vbyteEncode(v);
    println!("b: {:?}", b);
    let c = vbyteDecode(b);
    println!("c: {:?}", c);

}



#[cfg(test)]
#[test]
fn test_flatten() {

    let mut v = vec![];
    v.push((1, 2));
    v.push((3, 4));

    let flattened = v.iter().fold(Vec::new(), |mut v, c| {
        v.push(c.0);
        v.push(c.1);
        v
    });
    println!("flattened: {:?}", flattened);

}


#[cfg(test)]
#[test]
fn test_unsafe_flatten() {
    let mut v: Vec<(u32, u32)> = vec![];
    v.push((1, 2));
    v.push((3, 4));
    let n = v.len();
    let v2: Vec<u32> = unsafe {
        v.set_len(v.len() * 2);
        std::mem::transmute(v)
    };
    println!("v2: {:?}", v2);
}


#[cfg(test)]
#[test]
fn test_unsafe_flatten_reverse() {
    let mut v: Vec<(u32, u32)> = vec![];
    v.push((1, 2));
    v.push((3, 4));
    let n = v.len();
    println!("unsafe flatten reverse");
    println!("v: {:?}", v);
    let mut v2: Vec<u32> = unsafe {
        v.set_len(v.len() * 2);
        std::mem::transmute(v)
    };
    println!("v2: {:?}", v2);
    let v3: Vec<(u32, u32)> = unsafe {
        v2.set_len(v2.len() / 2);
        std::mem::transmute(v2)
    };
    println!("v3: {:?}", v3);
    println!("-----------------");

}
