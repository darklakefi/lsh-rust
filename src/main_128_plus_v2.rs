use solana_poseidon::{hashv, Endianness, Parameters};
use std::fs::OpenOptions;
use std::io::Write;

fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn hamming_distance_128(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn hamming_distance_string(a: &str, b: &str) -> u32 {
    a.chars()
        .zip(b.chars())
        .filter(|(char_a, char_b)| char_a != char_b)
        .count() as u32
}

fn normalize_vector(v: &Vec<f64>) -> Vec<f64> {
    let norm: f64 = v.iter().map(|&x| x * x).sum::<f64>().sqrt();
    
    if norm == 0.0 {
        v.clone() // Avoid division by zero
    } else {
        v.iter().map(|&x| x / norm).collect()
    }
}

fn split_u64_le(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

fn generate_lsh_rust(inputs: &[u64; 4]) -> String {
    let mut hash_res: String = "".to_string();

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);

    for i in 0..10 {
        let index_bytes = u64::to_le_bytes(i);

        let mut input_index = 0;
        let mut final_sum = 0;
        for &input in inputs.iter() {

            let input_index_bytes = u64::to_le_bytes(input_index);
            input_index += 1;

            let dim0: &[&[u8]] = &[&salt_bytes, &index_bytes, &input_index_bytes];

            let pos_hash0 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
            let pos_hash_bytes0 = pos_hash0.to_bytes();

            let mut array0 = [0u8; 8];
            array0.copy_from_slice(&pos_hash_bytes0[..8]);

            let projection = i64::from_le_bytes(array0);

            let mult0 = input as i128 * projection as i128;
            final_sum += mult0;
        }

        if final_sum < 0 {
            hash_res.push('1');
        } else {
            hash_res.push('0');
        }
    }
    

    hash_res
}


fn get_hash(is_swap_x_to_y: bool, balance_x: u64, balance_y: u64, input_amount: u64) -> String {
    let k = balance_x as u128 * balance_y as u128;

    let new_balance_x;
    let new_balance_y;
    let output;
    if (is_swap_x_to_y) {
        new_balance_x = balance_x + input_amount;
        new_balance_y = (k / new_balance_x as u128) as u64;
        output = balance_y - new_balance_y;
    } else {
        new_balance_y = balance_y + input_amount;
        new_balance_x = (k / new_balance_y as u128) as u64;
        output = balance_x - new_balance_x;
    }

    // println!("output: {} | new_balance_x: {} | new_balance_y: {}", output, new_balance_x, new_balance_y);

    // salt is 0 for now
    let input_vector: [u64; 4] = [balance_x, balance_y, new_balance_x, new_balance_y]; // Example input
    let lsh_hash = generate_lsh_rust(&input_vector);
    lsh_hash
}

fn fake_trade_to_y(balance_x: u64, balance_y: u64, input_amount: u64) -> (u64, u64) {
    let k = balance_x as u128 * balance_y as u128;

    let new_balance_x = balance_x + input_amount;
    let new_balance_y = (k / new_balance_x as u128) as u64;

    (new_balance_x, new_balance_y)
}

fn fake_trade_to_x(balance_x: u64, balance_y: u64, input_amount: u64) -> (u64, u64) {
    let k = balance_x as u128 * balance_y as u128;

    let new_balance_y = balance_y + input_amount;
    let new_balance_x = (k / new_balance_y as u128) as u64;

    (new_balance_x, new_balance_y)
}

fn main() {
    let input_amount =  100000000000;
    let mut balance_x = 100000000000000;
    let mut balance_y = 200000000000000;

    // user trading to y direction
    let is_swap_x_to_y = true;
    let mut front_run_input= 100;

    for i in 0..10 {
        let base_hash = get_hash(is_swap_x_to_y, balance_x, balance_y, input_amount);

        let mut better_balance_x = balance_x;
        let mut better_balance_y = balance_y;
        let mut worse_balance_x = balance_x;
        let mut worse_balance_y = balance_y;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("hamming-128bit-plus-res-{}.csv", i))
            .unwrap();

        writeln!(file, "better,worse").unwrap();

        let mut csv: String = "".to_string();

        let mut last_better_hash = "".to_string();
        let mut last_worse_hash = "".to_string();

        for t in 0..10 {
            (better_balance_x, better_balance_y) = fake_trade_to_x(better_balance_x, better_balance_y, front_run_input);
            (worse_balance_x, worse_balance_y) = fake_trade_to_y(worse_balance_x, worse_balance_y, front_run_input);

            let better_hash = get_hash(is_swap_x_to_y, better_balance_x, better_balance_y, input_amount);
            let worse_hash = get_hash(is_swap_x_to_y, worse_balance_x, worse_balance_y, input_amount);

            if (better_hash != last_better_hash) {
                println!("is better equal: {} | is worse equal: {}", better_hash == last_better_hash, worse_hash == last_worse_hash);
            }

            let better_distance = hamming_distance_string(&base_hash, &better_hash);
            let worse_distance = hamming_distance_string(&base_hash, &worse_hash);

            last_better_hash = better_hash;
            last_worse_hash = worse_hash;
            
            csv.push_str(&format!("{},{}\n", better_distance, worse_distance));
        }
        
        writeln!(file, "{}", csv).unwrap();

        // reduce to total token reserves
        // balance_x /= 10;
        // balance_y /= 10;
        front_run_input *= 10;
    }
}
