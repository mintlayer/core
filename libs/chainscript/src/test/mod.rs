use crate::*;

// Test the interpreter on all 4-byte combinations of non-trivial opcodes.
#[test]
//#[ignore = "This test is too expensive to run by default"]
fn test_4opc_sequences() {
    use hex::FromHex;
    use std::io::{BufRead, BufReader};

    // The test vectors are encoded in a gzipped CSV file.
    // Each line in the file is has the following comma-separated filelds:
    // 1) The hex-encoded bitcoin script
    // 2) The expected outcome, which is either 0 (script should fail) or 1 (script should succceed)
    // 3) If the expected outcome is 1 (success), then a sequence of comma-separated hex-encoded
    //    stack items after the execution of the script follows.
    //
    // The test vectors were obtained obtained by running the script interpreter in Tapscript mode
    // on all 4-opcode sequences of a subset of opcodes. Notable omissions include:
    // * `OP_NOP_N`, `OP_SUCCESS_N`: These are trivial and including them would make the file much
    //   larger (and test run time much longer) with little benefit.
    // * Opcodes dealing with checking signatures: These behave differently in Bitcoin.
    // * `OP_PUSHDATA_N`: Some these should be included in the future here or in a separate test.
    let test_vectors_raw = include_bytes!("test_vectors_4opc.csv.gz").as_ref();
    let test_vectors = BufReader::new(flate2::bufread::GzDecoder::new(test_vectors_raw));

    let mut fails = 0u32;
    for line in test_vectors.lines().map(|l| l.expect("can't get a line")) {
        let mut parts = line.split(',');
        // Load the script.
        let script: Script = Vec::from_hex(parts.next().expect("no script"))
            .expect("script not in hex format")
            .into();

        // Load the expected outcome. It should be either 0, or 1 followed by stack items.
        let expected: Option<Stack> = match parts.next().expect("no desired outcome") {
            "0" => None,
            "1" => {
                // For successful outcome, read the expected sequence of items on the stack
                let stack: Vec<_> =
                    parts.map(|s| Vec::from_hex(s).expect("hex item").into()).collect();
                Some(stack.into())
            }
            _ => unreachable!("bad format"),
        };

        // Run the script and record mismatches between expected and actual outputs.
        let result = run_script(&TestContext::default(), &script, vec![].into()).ok();
        if expected != result {
            eprintln!("FAIL {:?}: {:?} vs. {:?}", script, result, expected);
            fails += 1;
        }
    }

    // Let the test fail if we have at least one mismatch.
    assert!(fails == 0, "{} tests failed", fails);
}
