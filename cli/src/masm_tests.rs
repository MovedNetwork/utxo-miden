//! Module for writing tests for masm programs.

use miden::{
    math::Felt, Assembler, DefaultHost, ExecutionTrace, MemAdviceProvider, ProgramAst,
    ProvingOptions, StackInputs,
};
use miden_core::{StarkField, WORD_SIZE};
use miden_processor::AdviceProvider;
use miden_stdlib::StdLibrary;
use std::{collections::BTreeMap, fmt::Write, str::FromStr};

use crate::{
    advice_provider::UtxoAdvice,
    cli::prove,
    utils::HexString,
    utxo::{
        Key, SerializedTransaction, SerializedUtxo, SignedTransaction, State, Transaction, Utxo,
    },
};

// Tries running the prover on a real UTXO state and transaction
#[test]
fn test_main() {
    let key = Key::random().unwrap();
    let owner = key.owner;
    let initial_utxo = Utxo {
        owner,
        value: Felt::new(100),
    };
    let mut initial_state = State::empty();
    initial_state.insert(initial_utxo.clone()).unwrap();

    let output_1 = Utxo {
        owner,
        value: Felt::new(10),
    };
    let output_2 = Utxo {
        owner,
        value: Felt::new(90),
    };

    let transaction = Transaction {
        input: initial_utxo.hash(),
        outputs: vec![output_1, output_2],
    };
    let signed_tx = SignedTransaction::new(transaction.clone(), key.pair).unwrap();

    let stack_inputs = prove::prepare_stack_inputs(&initial_state, &signed_tx);
    let advice_provider = UtxoAdvice::new(&initial_state, signed_tx).unwrap();

    let trace = run_test(
        "../masm/utxo.masm",
        "main",
        stack_inputs,
        advice_provider,
        BTreeMap::default(),
    )
    .unwrap();
    // The top 4 elements on the stack represents the state root in reverse
    let mut stack_outputs = trace.stack_outputs().stack()[0..4].to_vec();
    stack_outputs.reverse();

    // Re-run the transaction in Rust implementation equivalent to compare the results
    let signed_tx = SignedTransaction::new(transaction.clone(), key.pair).unwrap();
    initial_state.process_tx(signed_tx).unwrap();
    let state_root = initial_state
        .get_root()
        .into_iter()
        .map(|el| el.as_int())
        .collect::<Vec<u64>>();
    assert_eq!(state_root, stack_outputs);
}

#[test]
fn test_divmod() {
    fn test_case(x: u32, y: u32) {
        let expected_remainder = (x % y) as u64;
        let expected_quotient = (x / y) as u64;

        let inputs = StackInputs::try_from_values([x as u64, y as u64]).unwrap();
        let trace = run_test(
            "../masm/utxo.masm",
            "divmod",
            inputs,
            MemAdviceProvider::default(),
            BTreeMap::new(),
        )
        .unwrap();
        let outputs = trace.stack_outputs();
        assert_eq!(
            &outputs.stack()[0..2],
            &[expected_remainder, expected_quotient]
        );
        assert!(outputs.stack().iter().skip(2).all(|x| *x == 0));
    }

    test_case(1, 1);
    test_case(1, 2);
    test_case(1, 3);
    test_case(2, 1);
    test_case(3, 1);
    test_case(5, 10);
    test_case(10, 5);
    test_case(101, 4);
    test_case(103, 4);
}

#[test]
fn test_range_hash() {
    fn test_case(input: &str, utxos: Vec<(&str, &str)>) {
        let transaction = Transaction::try_from(SerializedTransaction {
            input: HexString::from_str(input).unwrap(),
            outputs: utxos
                .iter()
                .map(|(owner, value)| SerializedUtxo {
                    owner: HexString::from_str(owner).unwrap(),
                    value: HexString::from_str(value).unwrap(),
                })
                .collect(),
        })
        .unwrap();

        let hash = transaction
            .hash()
            .iter()
            .map(|u| u.as_int())
            .collect::<Vec<u64>>();

        let number_of_elements_to_hash = 4 + utxos.len() as u64 * 5;
        let inputs = StackInputs::try_from_values([20, number_of_elements_to_hash]).unwrap();
        // Fill in the memory with transaction field elements contiguously in chunks of WORD SIZE
        let mut memory = BTreeMap::new();
        memory.insert(
            20,
            transaction
                .input
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>(),
        );
        // UTXO hashes each have 5 field elements (4 for hash + 1 for value).
        // We collect them first and fill in with zeros to make it a multiple of WORD SIZE.
        // Then we distribute these elements in incremental memory addresses.
        let mut utxo_felts = vec![];
        transaction.outputs.iter().for_each(|utxo| {
            utxo.owner.iter().for_each(|felt| {
                utxo_felts.push(felt.to_string());
            });
            utxo_felts.push(utxo.value.to_string());
        });
        while utxo_felts.len() % WORD_SIZE > 0 {
            utxo_felts.push("0".into());
        }
        utxo_felts.chunks(WORD_SIZE).for_each(|chunk| {
            memory.insert(20 + memory.len(), chunk.into());
        });

        let trace = run_test(
            "../masm/utxo.masm",
            "range_hash",
            inputs,
            MemAdviceProvider::default(),
            memory,
        )
        .unwrap();

        let mut outputs = trace.stack_outputs().stack().to_vec();
        outputs.truncate(WORD_SIZE);
        outputs.reverse(); // Stack is always returned in reverse
        assert_eq!(outputs, hash);
    }

    // tx_1
    test_case(
        "0xc039faf939fe7908f959dd5da871658e1b3f48998e9bd5165eb5acce45764fbb",
        vec![
            (
                "0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c",
                "0xf000000000000000",
            ),
            (
                "0x496d5921189e0f6a49b64c90a62286a47381bd63641ef9d847f7b9fc917b68f8",
                "0x0f00000000000000",
            ),
        ],
    );
    // tx_2
    test_case(
        "0x42007c73912db5a323312160d24008aacd6a25d38b3f30adb37dca7304e46347",
        vec![(
            "0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c",
            "0x0f00000000000000",
        )],
    );
    // tx_3
    test_case(
        "0xeca9699210de0ecf6764f1dde94410e142645796170ac016dc22dc6a3c84b1db",
        vec![],
    );
}

fn run_test<A: AdviceProvider>(
    masm_source_path: &str,
    proc_name: &str,
    stack_inputs: StackInputs,
    advice_provider: A,
    memory: BTreeMap<usize, Vec<String>>,
) -> anyhow::Result<ExecutionTrace> {
    let code = std::fs::read_to_string(masm_source_path)?;
    let ast = ProgramAst::parse(&code)?;

    let main_start = ast
        .body()
        .source_locations()
        .first()
        .ok_or_else(|| anyhow::Error::msg("No start source location"))?
        .line() as usize
        - 2;
    let main_end = ast
        .body()
        .source_locations()
        .last()
        .ok_or_else(|| anyhow::Error::msg("No end source location"))?
        .line() as usize;

    // Simple masm code to run the procedure we want.
    let memory_code = memory
        .iter()
        .map(|(index, elements)| {
            assert_eq!(elements.len(), WORD_SIZE);
            format!(
                "  push.{}\n  mem_storew.{index}\n  dropw\n",
                elements.join(".")
            )
        })
        .collect::<Vec<String>>()
        .join("");

    let test_harness = format!("begin\n{memory_code}\n  exec.{proc_name}\nend");
    let test_code = code
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            if (main_start..=main_end).contains(&i) {
                None
            } else {
                Some(line)
            }
        })
        .chain(test_harness.lines())
        .fold(String::new(), |mut acc, l| {
            writeln!(&mut acc, "{l}").unwrap();
            acc
        });

    let assembler = Assembler::default().with_library(&StdLibrary::default())?;
    let program = assembler.compile(test_code)?;
    let host = DefaultHost::new(advice_provider);
    let trace = miden::execute(
        &program,
        stack_inputs,
        host,
        *ProvingOptions::default().execution_options(),
    )?;

    Ok(trace)
}
