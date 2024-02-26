use solana_program_test::{processor, ProgramTest};

mod validate_contract_call;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gmp_gateway",
        gmp_gateway::id(),
        processor!(gmp_gateway::processor::Processor::process_instruction),
    )
}