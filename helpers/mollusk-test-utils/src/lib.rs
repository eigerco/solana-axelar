use anchor_lang::prelude::UpgradeableLoaderState;
use mollusk_svm::Mollusk;
use solana_sdk::{account::Account, pubkey::Pubkey, rent::Rent};
use solana_sdk_ids::{bpf_loader_upgradeable, system_program};

pub fn setup_mollusk(program_id: &Pubkey, program_name: &str) -> Mollusk {
    std::env::set_var("SBF_OUT_DIR", "../../target/deploy");
    Mollusk::new(program_id, program_name)
}

// TODO(v2) use create_program_data_account_loader_v3 once it supports
// setting the upgrade authority
// Inspired by https://github.com/anza-xyz/mollusk/blob/1cfdd642b3afa351068148d008c0b4f066ed09c6/harness/src/program.rs#L305
#[allow(clippy::indexing_slicing)]
pub fn create_program_data_account(elf: &[u8], upgrade_authority: Pubkey) -> Account {
    let data = {
        let elf_offset = UpgradeableLoaderState::size_of_programdata_metadata();
        let data_len = elf_offset + elf.len();
        let mut data = vec![0; data_len];
        bincode::serialize_into(
            &mut data[0..elf_offset],
            &UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: Some(upgrade_authority),
            },
        )
        .expect("Failed to serialize program data account");
        data[elf_offset..].copy_from_slice(elf);
        data
    };
    let lamports = Rent::default().minimum_balance(data.len());

    Account {
        lamports,
        data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    }
}

pub fn get_event_authority_and_program_accounts(program_id: &Pubkey) -> (Pubkey, Account, Account) {
    let (event_authority, _bump) =
        Pubkey::find_program_address(&[b"__event_authority"], program_id);
    let event_authority_account = Account::new(0, 0, &system_program::ID);

    let program_account = mollusk_svm::program::create_program_account_loader_v3(program_id);

    (event_authority, event_authority_account, program_account)
}
