//! Module for writing tests for masm programs.

use miden::{
    AdviceInputs, Assembler, DefaultHost, ExecutionTrace, MemAdviceProvider, ProgramAst,
    ProvingOptions, StackInputs,
};
use miden_stdlib::StdLibrary;
use std::fmt::Write;

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
            AdviceInputs::default(),
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

// TODO: include parameter to initialize memory
fn run_test(
    masm_source_path: &str,
    proc_name: &str,
    stack_inputs: StackInputs,
    advice_inputs: AdviceInputs,
) -> anyhow::Result<ExecutionTrace> {
    let code = std::fs::read_to_string(masm_source_path)?;
    let ast = ProgramAst::parse(&code)?;
    let proc = ast
        .procedures()
        .iter()
        .find(|p| p.name.as_str() == proc_name)
        .ok_or_else(|| anyhow::Error::msg("Procedure not found"))?;
    let start = proc
        .source_locations()
        .next()
        .ok_or_else(|| anyhow::Error::msg("No start source location"))?
        .line() as usize;
    let end = proc
        .source_locations()
        .last()
        .ok_or_else(|| anyhow::Error::msg("No end source location"))?
        .line() as usize;

    // Simple masm code to run the procedure we want.
    // TODO: include code to initialize memory
    let test_harness = format!("begin\n  exec.{proc_name}\nend");

    let test_code = code
        .lines()
        .skip(start - 1)
        .take(end - start + 1)
        .chain(test_harness.lines())
        .fold(String::new(), |mut acc, l| {
            writeln!(&mut acc, "{l}").unwrap();
            acc
        });

    let assembler = Assembler::default().with_library(&StdLibrary::default())?;
    let program = assembler.compile(test_code)?;
    let host = DefaultHost::new(MemAdviceProvider::from(advice_inputs));
    let trace = miden::execute(
        &program,
        stack_inputs,
        host,
        ProvingOptions::default().exec_options,
    )?;

    Ok(trace)
}
