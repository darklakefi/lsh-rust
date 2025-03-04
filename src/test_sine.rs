use num_bigint::BigInt;
use rand::distr::weighted;
use solana_poseidon::{hashv, Endianness, Parameters};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::f64::consts::PI;


fn main() {
    let freq = 0.5;
    let mut time = 1.0;
    
    // println!("{}", (PI / 2.0).sin());    
    
    for i in 0..1000 {
        let a = (2.0 * PI * freq * time).sin();
    
        println!("{}", a);    
        time += 0.0001;
    }
}