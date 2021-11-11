#[cfg(test)]
#[test]
fn test_vbyte() {
    use crate::vbyte::*;
    let a = vbyteEncodeNumber(1024);
    println!("{:?}", a);

    let v = vec![1, 2, 3, 1024];
    let b = vbyteEncode(v.clone());
    println!("b: {:?}", b);
    let c = vbyteDecode(b);
    println!("c: {:?}", c);
    assert_eq!(v, c);

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
    use crate::vbyte::*;

    let mut v: Vec<(u32, u32)> = vec![];
    v.push((1, 2));
    v.push((3, 4));
    let vv = v.clone();

    let mut v2: Vec<u32> = unsafe {
        v.set_len(v.len() * 2);
        std::mem::transmute(v)
    };

    let bytes = vbyteEncode(v2.clone());
    println!("bytes: {:?}", bytes);
    let mut v3 = vbyteDecode(bytes);
    assert_eq!(v2, v3);

    let v4: Vec<(u32, u32)> = unsafe {
        v3.set_len(v3.len() / 2);
        std::mem::transmute(v3)
    };
    assert_eq!(vv, v4);

}
