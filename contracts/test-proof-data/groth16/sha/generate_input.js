#!/usr/bin/env node

// Your 16 integers (each up to 2^32 - 1).
// These match the ones in your original input.json.
const inputNums = [
    1819043144,
    1864398703,
    1114795884,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    88
  ];
  
  /**
   * Convert a 32-bit unsigned integer into an array of 32 bits (as 0/1 numbers).
   * LSB (least significant bit) is bitArray[0], MSB is bitArray[31].
   */
  function toBits32(num) {
    const bits = [];
    for (let i = 0; i < 32; i++) {
      bits.push((num >> i) & 1);
    }
    return bits;
  }
  
  // Expand the 16 integers into a flat array of 512 bits.
  let bitArray = [];
  for (let i = 0; i < inputNums.length; i++) {
    // Convert each 32-bit integer to 32 bits.
    const bitsForNum = toBits32(inputNums[i]);
  
    // If you want MSB first, uncomment the next line:
    // bitsForNum.reverse();
  
    bitArray = bitArray.concat(bitsForNum);
  }
  
  // Prepare JSON with an array of 512 bits as strings ("0" or "1").
  const inputJson = {
    in: bitArray.map(String)
  };
  
  console.log(JSON.stringify(inputJson, null, 2));
  