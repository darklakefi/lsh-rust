use solana_poseidon::{hashv, Endianness, Parameters};
use std::fs::OpenOptions;
use std::io::Write;

fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

// written to match circuit output, but not following circuit specific math
fn generate_lsh_rust(input: &[u64; 3]) -> u64 {
    let mut hash_res: u64 = 0;

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);


    for i in 0..64 {
        let index_bytes = u64::to_le_bytes(i);
        let zero_bytes = u64::to_le_bytes(0);
        let one_bytes = u64::to_le_bytes(1);
        let two_bytes = u64::to_le_bytes(2);

        let dim0: &[&[u8]] = &[&salt_bytes, &index_bytes, &zero_bytes];
        let dim1: &[&[u8]] = &[&salt_bytes, &index_bytes, &one_bytes];
        let dim2: &[&[u8]] = &[&salt_bytes, &index_bytes, &two_bytes];

        let pos_hash0 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
        let pos_hash1 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim1).unwrap();
        let pos_hash2 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim2).unwrap();
    
        let pos_hash_bytes0 = pos_hash0.to_bytes();
        let pos_hash_bytes1 = pos_hash1.to_bytes();
        let pos_hash_bytes2 = pos_hash2.to_bytes();
        
        let mut array0 = [0u8; 8];
        let mut array1 = [0u8; 8];
        let mut array2 = [0u8; 8];

        array0.copy_from_slice(&pos_hash_bytes0[..8]);
        array1.copy_from_slice(&pos_hash_bytes1[..8]);
        array2.copy_from_slice(&pos_hash_bytes2[..8]);

        let projection: [i64; 3] = [
            i64::from_le_bytes(array0),
            i64::from_le_bytes(array1),
            i64::from_le_bytes(array2)
        ];

        let mult0: i128 = input[0] as i128 * projection[0] as i128;
        let mult1: i128 = input[1] as i128 * projection[1] as i128;
        let mult2: i128 = input[2] as i128 * projection[2] as i128;

        let final_sum = mult0 + mult1 + mult2;

        if final_sum < 0 {
            hash_res |= 1 << 63-i;
        }
    }

    hash_res
}


// Function written a way to mimic the zk circuit math
fn generate_lsh_circuit(input: &[u64; 3]) -> u64 {
    let mut hash_res: u64 = 0;

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);


    for i in 0..64 {
        let index_bytes = u64::to_le_bytes(i);
        let zero_bytes = u64::to_le_bytes(0);
        let one_bytes = u64::to_le_bytes(1);
        let two_bytes = u64::to_le_bytes(2);

        let dim0: &[&[u8]] = &[&salt_bytes, &index_bytes, &zero_bytes];
        let dim1: &[&[u8]] = &[&salt_bytes, &index_bytes, &one_bytes];
        let dim2: &[&[u8]] = &[&salt_bytes, &index_bytes, &two_bytes];

        let pos_hash0 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
        let pos_hash1 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim1).unwrap();
        let pos_hash2 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim2).unwrap();
    
        let pos_hash_bytes0 = pos_hash0.to_bytes();
        let pos_hash_bytes1 = pos_hash1.to_bytes();
        let pos_hash_bytes2 = pos_hash2.to_bytes();
        
        let mut array0 = [0u8; 8];
        let mut array1 = [0u8; 8];
        let mut array2 = [0u8; 8];

        array0.copy_from_slice(&pos_hash_bytes0[..8]);
        array1.copy_from_slice(&pos_hash_bytes1[..8]);
        array2.copy_from_slice(&pos_hash_bytes2[..8]);

        let mut projection: [u64; 3] = [
            u64::from_le_bytes(array0),
            u64::from_le_bytes(array1),
            u64::from_le_bytes(array2)
        ];

        let mut isProjectionNegative = [false; 3];

        for (j, proj) in projection.iter_mut().enumerate() {
            if *proj >= 9223372036854775808 {
                *proj -= 9223372036854775808;
                *proj = 9223372036854775808 - *proj;
                isProjectionNegative[j] = true;
            } else {
                isProjectionNegative[j] = false;
            }
        }

        let mult0: u128 = input[0] as u128 * projection[0] as u128;
        let mult1: u128 = input[1] as u128 * projection[1] as u128;
        let mult2: u128 = input[2] as u128 * projection[2] as u128;

        let is_mult0_negative= isProjectionNegative[0];
        let is_mult1_negative= isProjectionNegative[1];
        let is_mult2_negative= isProjectionNegative[2];

        // assign initial values
        let mut final_sum = mult0;
        let mut is_final_negative = is_mult0_negative;

        if (is_final_negative != is_mult1_negative && final_sum < mult1) {
            final_sum = mult1 - final_sum;
            is_final_negative = is_mult1_negative;
        } else if (is_final_negative != is_mult1_negative) {
            final_sum -= mult1;
        } else {
            final_sum += mult1;
        }

        if (is_final_negative != is_mult2_negative && final_sum < mult2) {
            final_sum = mult2 - final_sum;
            is_final_negative = is_mult2_negative;
        } else if (is_final_negative != is_mult2_negative) {
            final_sum -= mult2;
        } else {
            final_sum += mult2;
        }

        if is_final_negative {
            hash_res |= 1 << 63-i;
        }
    }

    hash_res
}

fn get_hash(is_swap_x_to_y: bool, balance_x: u64, balance_y: u64, input_amount: u64, is_v2: bool) -> u64 {
    let k = balance_x * balance_y;

    let new_balance_x;
    let new_balance_y;
    let output;
    if (is_swap_x_to_y) {
        new_balance_x = balance_x + input_amount;
        new_balance_y = k / new_balance_x;
        output = balance_y - new_balance_y;
    } else {
        new_balance_y = balance_y + input_amount;
        new_balance_x = k / new_balance_y;
        output = balance_x - new_balance_x;
    }

    // salt is 0 for now
    let input_vector: [u64; 3] = [new_balance_x, new_balance_y, output]; // Example input
    let lsh_hash = if is_v2 { generate_lsh_rust(&input_vector) } else { generate_lsh_circuit(&input_vector) };
    lsh_hash
}

fn fake_trade_to_y(balance_x: u64, balance_y: u64, input_amount: u64) -> (u64, u64) {
    let k = balance_x * balance_y;

    let new_balance_x = balance_x + input_amount;
    let new_balance_y = k / new_balance_x;

    (new_balance_x, new_balance_y)
}

fn fake_trade_to_x(balance_x: u64, balance_y: u64, input_amount: u64) -> (u64, u64) {
    let k: u64 = balance_x * balance_y;

    let new_balance_y = balance_y + input_amount;
    let new_balance_x = k / new_balance_y;

    (new_balance_x, new_balance_y)
}

fn main() {
    let input_amount = 100;
    let mut balance_x = 1000000;
    let mut balance_y = 2000000;

    // user trading to y direction
    let is_swap_x_to_y = true;

    for i in 0..3 {
        let base_hash = get_hash(is_swap_x_to_y, balance_x, balance_y, input_amount, false);

        let mut better_balance_x = balance_x;
        let mut better_balance_y = balance_y;
        let mut worse_balance_x = balance_x;
        let mut worse_balance_y = balance_y;


        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("hamming-64bit-{}.csv", i))
            .unwrap();

        writeln!(file, "better,worse").unwrap();

        let mut csv: String = "".to_string();

        for t in 0..64 {
            (better_balance_x, better_balance_y) = fake_trade_to_x(better_balance_x, better_balance_y, 100);
            (worse_balance_x, worse_balance_y) = fake_trade_to_y(worse_balance_x, worse_balance_y, 100);

            let better_hash = get_hash(is_swap_x_to_y, better_balance_x, better_balance_y, input_amount, false);
            let worse_hash = get_hash(is_swap_x_to_y, worse_balance_x, worse_balance_y, input_amount, false);
            
            let better_distance = hamming_distance(base_hash, better_hash);
            let worse_distance = hamming_distance(base_hash, worse_hash);

            csv.push_str(&format!("{},{}\n", better_distance, worse_distance));
        }

        writeln!(file, "{}", csv).unwrap();

        // reduce to total token reserves
        balance_x /= 10;
        balance_y /= 10;
    }
}
