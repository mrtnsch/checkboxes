pub fn bitmap_to_tuple(bitmap: Vec<u8>) -> (Vec<usize>, Vec<usize>) {
    // Create two vectors to store the indices of true and false bits
    let mut true_indices: Vec<usize> = Vec::new();
    let mut false_indices: Vec<usize> = Vec::new();

    // Convert each byte to a binary representation and store indices
    for (byte_index, byte) in bitmap.iter().enumerate() {
        for bit_index in 0..8 {
            // Calculate the bit's position in the overall bitmap
            let bit_position = byte_index * 8 + bit_index;
            // Extract the bit value and classify it
            if (byte >> (7 - bit_index)) & 1 == 1 {
                true_indices.push(bit_position);
            } else {
                false_indices.push(bit_position);
            }
        }
    }
    return (true_indices, false_indices);
}