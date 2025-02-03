const { buildPoseidon } = require("circomlibjs");

async function main() {
  const poseidon = await buildPoseidon();
  const F = poseidon.F;

  // Our trivial example
  const leaf = 1n;

  // 32 zeros for siblings
  const levels = 32;
  const pathElements = new Array(levels).fill(0n);

  // 32 zeros for pathIndices => always leaf on the left, sibling on the right
  const pathIndices = new Array(levels).fill(0);

  // Compute "root" by the same logic as the circuit:
  let current = leaf;
  for (let i = 0; i < levels; i++) {
    // If pathIndex=0 => (current, sibling) => Poseidon([current, sibling])
    // If pathIndex=1 => (sibling, current)
    // But here pathIndex=0 every time => always Poseidon([current, 0])
    current = F.toObject(poseidon([current, 0n]));
  }
  const root = current;

  // Build the JSON
  const inputData = {
    leaf: leaf.toString(),
    root: root.toString(),
    pathElements: pathElements.map(x => x.toString()),
    pathIndices: pathIndices
  };

  // Write the file
  const fs = require("fs");
  fs.writeFileSync("input.json", JSON.stringify(inputData, null, 2));

  console.log("Wrote input.json:");
  console.log(JSON.stringify(inputData, null, 2));
}

main().catch(console.error);
