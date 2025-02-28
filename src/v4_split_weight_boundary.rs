use solana_poseidon::{hashv, Endianness, Parameters};
use std::fs::OpenOptions;
use std::io::Write;
use num_bigint::BigInt;

fn hamming_distance_string(a: &str, b: &str) -> u32 {
    a.chars()
        .zip(b.chars())
        .filter(|(char_a, char_b)| char_a != char_b)
        .count() as u32
}

fn split_u64_into_weighted_nibbles(value: u64, max_bits: u32, cut_off_index: u32) -> Vec<u64> {
    // println!("value: {} | max_bits: {} | cut_off_index: {}", value, max_bits, cut_off_index);

    assert!(max_bits <= 64, "max_bits must be between 0 and 64");

    let num_chunks = (max_bits + 3) / 4; // Number of 4-bit segments
    let max_bits_rounded = num_chunks * 4;
    let mut results = Vec::with_capacity(num_chunks as usize);

    // println!("num_chunks: {}", num_chunks);

    for i in 0..num_chunks {
        if (i < cut_off_index) {
            results.push(0);
            continue;
        }

        let shift = (i) * 4; // Compute bit shift from MSB
        // println!("i: {} | shift {}", i, shift);

        let nibble = if shift >= 0 {
            (value >> shift) & 0xF // Extract 4-bit segment
        } else {
            0 // If out of range, use 0
        };

        // uncomment which type of weight increase to used (x1, x2, x3... or x1, x2, x4...)
        // let weight = 1 << (num_chunks - (i - cut_off_index)); // Weight = 2^i
        let weight = ((i + 1) - cut_off_index) as u64;      // Weight = i

        // if nibble == 0 {
        //     results.push(1);
        // } else {
        results.push((nibble) * weight);
        // }
    }

    results
}


fn generate_lsh_rust(inputs: &[u64; 1]) -> String {
    let mut hash_res: String = "".to_string();

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);

    let lsh_inputs = inputs;

    // projection count (generally recommended 512/1024 for higher precision but lower performance)
    for i in 0..128 {
        let index_bytes = u64::to_le_bytes(i);

        let mut input_index = 5000;
        let mut final_sum: BigInt = BigInt::from(0);
        let max_bits = 64;

        for &input in lsh_inputs.iter() {


            // cutoff currently not used (part of previous experiments), drops some amount of LSB bits
            let mut input_parts = split_u64_into_weighted_nibbles(input, max_bits, 0);
            println!("input_parts: {:?}", input_parts);

            for &input_u8 in input_parts.iter() {
                let input_index_bytes = u64::to_le_bytes(input_index);
                input_index += 1;
    
                let dim0: &[&[u8]] = &[&salt_bytes, &index_bytes, &input_index_bytes];
    
                let pos_hash0 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
                let pos_hash_bytes0 = pos_hash0.to_bytes();
    
                let mut array0 = [0u8; 8];
                array0.copy_from_slice(&pos_hash_bytes0[..8]);

                let projection = i64::from_le_bytes(array0);
    
                let mult0 = input_u8 as i128 * projection as i128;
                // println!(" projection: {} | input: {} | mult0: {} |", projection, input, mult0);
                final_sum += mult0;
                // println!("final_sum: {}", final_sum);
            }
        }

        if final_sum.lt(&BigInt::from(0)) {
            hash_res.push('1');
        } else {
            hash_res.push('0');
        }
    }
    
    hash_res
}


// calculates the k=x*y constant and returns the hash and the received amount
fn get_hash(is_swap_x_to_y: bool, balance_x: u64, balance_y: u64, input_amount: u64) -> (String, u64) {
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
    let input_vector = [output]; // Example input

    // let allowed = input_amount * ; // * percentage / 10000 (max)
    let lsh_hash = generate_lsh_rust(&input_vector);
    return (lsh_hash, output)
}

// slippage = 10,000 = 100%
// sames as get_hash but returns two hashes and two outputs (and the original "center" output)
fn get_boundary_hashes(is_swap_x_to_y: bool, balance_x: u64, balance_y: u64, input_amount: u64, slippage: u64) -> ([String; 2], u64, u64, u64) {
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
    // let input_vector: [u64; 1] = []; // Example input
    let upper_output = output + output * slippage / 10000;
    let lower_output = output - output * slippage / 10000;  

    let upper_outputs = [output + output * slippage / 10000];
    let lower_outputs = [output - output * slippage / 10000];
    // let allowed = input_amount * ; // * percentage / 10000 (max)
    println!("Generating upper");
    let upper_lsh_hash = generate_lsh_rust(&upper_outputs);
    println!("lower upper");
    let lower_lsh_hash = generate_lsh_rust(&lower_outputs);
    
    return ([upper_lsh_hash, lower_lsh_hash], upper_output, lower_output, output);
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
    let input_amount =  10000000;
    let mut balance_x = 10000000000000;
    let mut balance_y = 80000000000000;

    let slippage = 200; // 5% (10,000 = 100%)

    // user trading to y direction
    let is_swap_x_to_y = true;
    let mut front_run_base= 100;

    let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    // .open(format!("v3-split-weight-high-high-2-2.csv"))
    .open(format!("v4-split-weight-bound-1-8.csv"))
    .unwrap();

    // get two LSH hashes of boundaries arround the current receive token amount (+/- slippage)
    let ([base_upper_hash, base_lower_hash], base_upper_output, base_lower_output, base_output) = get_boundary_hashes(is_swap_x_to_y, balance_x, balance_y, input_amount, slippage);
    println!("generating base");
    println!("base_upper_output: {}", base_upper_output);
    println!("base_lower_output: {}", base_lower_output);
    println!("base_output:       {}", base_output);
    
    // get LSH hash of the receive token amount
    let (base_hash, base_output_2) = get_hash(is_swap_x_to_y, balance_x, balance_y, input_amount);

    let base_hash_distance = hamming_distance_string(&base_upper_hash, &base_lower_hash);
    let to_upper = hamming_distance_string(&base_hash, &base_upper_hash);
    let to_lower = hamming_distance_string(&base_hash, &base_lower_hash);
    
    writeln!(file, "base_distance: {} | base_upper_output: {} | base_lower_output: {}", base_hash_distance, base_upper_output, base_lower_output).unwrap();
    writeln!(file, "base_to_up: {} | base_to_low: {}", to_upper, to_lower).unwrap();
    writeln!(file, "upper_hash: {}", base_upper_hash).unwrap();
    writeln!(file, "base_hash:  {}", base_hash).unwrap();
    writeln!(file, "low_hash:   {}", base_lower_hash).unwrap();


    // loop for major front-run token amount increase (front_run_base_amount = front_run_base_amount*10)
    for i in 0..12 {
        println!("---Balance RESET---");
        let mut front_run_input = front_run_base;

        writeln!(file, "{}-{}", front_run_input, front_run_input*9).unwrap();
        writeln!(file, "better_up,better_low,worse_up,worse_low,better_up_crossed,better_low_crossed,worse_up_crossed,worse_low_crossed,better_perc,worse_perc").unwrap();

        let mut csv: String = "".to_string();

        // loop for minor front-run token amount increase (front_run = front_run + front_run_base_amount)
        for t in 0..9 {
            println!("front_run_input: {}", front_run_input);

            let (better_balance_x, better_balance_y) = fake_trade_to_x(balance_x, balance_y, front_run_input);
            let (worse_balance_x, worse_balance_y) = fake_trade_to_y(balance_x, balance_y, front_run_input);

            let (better_hash, better_output) = get_hash(is_swap_x_to_y, better_balance_x, better_balance_y, input_amount);
            let (worse_hash, worse_output) = get_hash(is_swap_x_to_y, worse_balance_x, worse_balance_y, input_amount);

            let better_distance_to_upper = hamming_distance_string(&base_upper_hash, &better_hash);
            let better_distance_to_lower = hamming_distance_string(&base_lower_hash, &better_hash);

            let worse_distance_to_upper = hamming_distance_string(&base_upper_hash, &worse_hash);
            let worse_distance_to_lower = hamming_distance_string(&base_lower_hash, &worse_hash);
            
            csv.push_str(
                &format!(
                    "{},{},{},{},{},{},{},{},{},{},{},{}\n",
                    better_distance_to_upper,
                    better_distance_to_lower,
                    worse_distance_to_upper,
                    worse_distance_to_lower,
                    better_distance_to_upper >= base_hash_distance,
                    better_distance_to_lower >= base_hash_distance,
                    worse_distance_to_upper >= base_hash_distance,
                    worse_distance_to_lower >= base_hash_distance,
                    better_output,
                    worse_output,
                    ((better_output as f64 / base_output as f64) as f64 - 1.0) as f64,
                    (1.0 - (worse_output as f64 / base_output as f64) as f64) as f64,
                )
            );
            front_run_input += front_run_base;
        }
        
        writeln!(file, "{}", csv).unwrap();

        front_run_base = 100 * 10u64.pow(i + 1);
    }
}
