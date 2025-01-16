package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"os"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark-crypto/kzg"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/backend/plonk"
	"github.com/consensys/gnark/backend/witness"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/test/unsafekzg"
)

// ProverInput represents the JSON structure passed from Rust
type ProverInput struct {
	R1CS          []byte          `json:"r1cs"`
	PublicInputs  json.RawMessage `json:"public_inputs"`
	PrivateInputs json.RawMessage `json:"private_inputs"`
	SchemeConfig  string          `json:"scheme_config"`
	Curve         string          `json:"curve"`
	// Optional SRS data for PLONK
	SRS         []byte `json:"srs,omitempty"`
	SRSLagrange []byte `json:"srs_lagrange,omitempty"`
}

// ProofOutput represents the data returned to Rust
type ProofOutput struct {
	Proof           []byte `json:"proof"`
	PublicInputs    []byte `json:"public_inputs"`
	VerificationKey []byte `json:"verification_key"`
}

func main() {
	// Parse command line flags
	paramsPath := flag.String("params", "", "Path to params JSON file")
	outputPath := flag.String("output", "", "Path for proof output")
	flag.Parse()

	if *paramsPath == "" || *outputPath == "" {
		log.Fatal("Both --params and --output flags are required")
	}

	// Read and parse input params
	paramsData, err := os.ReadFile(*paramsPath)
	if err != nil {
		log.Fatalf("Failed to read params file: %v", err)
	}

	var input ProverInput
	if err := json.Unmarshal(paramsData, &input); err != nil {
		log.Fatalf("Failed to parse params JSON: %v", err)
	}

	// Get curve type
	var curveID ecc.ID
	switch input.Curve {
	case "bn254":
		curveID = ecc.BN254
	case "bls12-381":
		curveID = ecc.BLS12_381
	default:
		log.Fatalf("Unsupported curve: %s", input.Curve)
	}

	// Handle different proving schemes
	switch input.SchemeConfig {
	case "groth16":
		if err := handleGroth16(input, curveID, *outputPath); err != nil {
			log.Fatalf("Groth16 error: %v", err)
		}
	case "plonk":
		if err := handlePlonk(input, curveID, *outputPath); err != nil {
			log.Fatalf("PLONK error: %v", err)
		}
	default:
		log.Fatalf("Unsupported scheme: %s", input.SchemeConfig)
	}
}

func handleGroth16(input ProverInput, curveID ecc.ID, outputPath string) error {
	if curveID != ecc.BN254 {
		return fmt.Errorf("Groth16 only supports bn254 curve")
	}

	// Create new R1CS
	r1cs := groth16.NewCS(curveID)

	// Parse R1CS bytes
	if _, err := r1cs.ReadFrom(bytes.NewReader(input.R1CS)); err != nil {
		return fmt.Errorf("failed to parse R1CS: %v", err)
	}

	// Setup
	pk, vk, err := groth16.Setup(r1cs)
	if err != nil {
		return fmt.Errorf("setup error: %v", err)
	}

	// Create and fill witness
	w, err := createWitness(input, curveID, r1cs)
	if err != nil {
		return fmt.Errorf("witness error: %v", err)
	}

	// Generate proof
	proof, err := groth16.Prove(r1cs, pk, w)
	if err != nil {
		return fmt.Errorf("proving error: %v", err)
	}

	return writeOutput(proof, vk, input.PublicInputs, outputPath)
}

func handlePlonk(input ProverInput, curveID ecc.ID, outputPath string) error {
	// Create new constraint system
	r1cs := plonk.NewCS(curveID)

	// Parse R1CS bytes
	if _, err := r1cs.ReadFrom(bytes.NewReader(input.R1CS)); err != nil {
		return fmt.Errorf("failed to parse R1CS: %v", err)
	}

	var srs, srsLagrange kzg.SRS
	srs = kzg.NewSRS(curveID)
	srsLagrange = kzg.NewSRS(curveID)

	// If SRS data is provided, use it
	if len(input.SRS) > 0 && len(input.SRSLagrange) > 0 {
		// Parse provided SRS data
		if _, err := srs.ReadFrom(bytes.NewReader(input.SRS)); err != nil {
			return fmt.Errorf("failed to parse SRS: %v", err)
		}
		if _, err := srsLagrange.ReadFrom(bytes.NewReader(input.SRSLagrange)); err != nil {
			return fmt.Errorf("failed to parse SRS Lagrange: %v", err)
		}
	} else {
		// For testing/development: generate unsafe SRS
		// WARNING: This should not be used in production!
		srsTemp, srsLagrangeTemp, err := unsafekzg.NewSRS(r1cs)
		if err != nil {
			return fmt.Errorf("failed to create test SRS: %v", err)
		}
		srs = srsTemp
		srsLagrange = srsLagrangeTemp
	}

	// Setup
	pk, vk, err := plonk.Setup(r1cs, srs, srsLagrange)
	if err != nil {
		return fmt.Errorf("setup error: %v", err)
	}

	// Create and fill witness
	w, err := createWitness(input, curveID, r1cs)
	if err != nil {
		return fmt.Errorf("witness error: %v", err)
	}

	// Generate proof
	proof, err := plonk.Prove(r1cs, pk, w)
	if err != nil {
		return fmt.Errorf("proving error: %v", err)
	}

	return writeOutput(proof, vk, input.PublicInputs, outputPath)
}

func createWitness(input ProverInput, curveID ecc.ID, cs interface{}) (witness.Witness, error) {
	w, err := witness.New(curveID.ScalarField())
	if err != nil {
		return nil, fmt.Errorf("failed to create witness: %v", err)
	}

	var publicInputs, privateInputs map[string]interface{}
	if err := json.Unmarshal(input.PublicInputs, &publicInputs); err != nil {
		return nil, fmt.Errorf("failed to parse public inputs: %v", err)
	}
	if err := json.Unmarshal(input.PrivateInputs, &privateInputs); err != nil {
		return nil, fmt.Errorf("failed to parse private inputs: %v", err)
	}

	// Get number of variables based on constraint system type
	var nbPublic, nbSecret int
	switch cs := cs.(type) {
	case constraint.ConstraintSystem:
		nbPublic = cs.GetNbPublicVariables()
		nbSecret = cs.GetNbSecretVariables()
	default:
		return nil, fmt.Errorf("unsupported constraint system type")
	}

	// Fill witness with values
	values := make(chan interface{})
	go func() {
		defer close(values)
		for _, v := range publicInputs {
			values <- v
		}
		for _, v := range privateInputs {
			values <- v
		}
	}()

	if err := w.Fill(nbPublic, nbSecret, values); err != nil {
		return nil, fmt.Errorf("failed to fill witness: %v", err)
	}

	return w, nil
}

func writeOutput(proof, vk interface{}, publicInputs json.RawMessage, outputPath string) error {
	var proofBuf, vkBuf bytes.Buffer

	if p, ok := proof.(io.WriterTo); ok {
		if _, err := p.WriteTo(&proofBuf); err != nil {
			return fmt.Errorf("failed to serialize proof: %v", err)
		}
	} else {
		return fmt.Errorf("proof does not implement WriterTo")
	}

	if v, ok := vk.(io.WriterTo); ok {
		if _, err := v.WriteTo(&vkBuf); err != nil {
			return fmt.Errorf("failed to serialize verification key: %v", err)
		}
	} else {
		return fmt.Errorf("verification key does not implement WriterTo")
	}

	output := ProofOutput{
		Proof:           proofBuf.Bytes(),
		PublicInputs:    publicInputs,
		VerificationKey: vkBuf.Bytes(),
	}

	outputFile, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("failed to create output file: %v", err)
	}
	defer outputFile.Close()

	if err := json.NewEncoder(outputFile).Encode(output); err != nil {
		return fmt.Errorf("failed to write output: %v", err)
	}

	return nil
}
