use anchor_lang::{
    system_program::ID as SYSTEM_PROGRAM_ID,
    InstructionData, ToAccountMetas,
};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        default_account_state::DefaultAccountState,
        metadata_pointer::MetadataPointer,
        mint_close_authority::MintCloseAuthority,
        transfer_fee::TransferFeeConfig,
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::{AccountState, Mint},
};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use anchor_lang::solana_program::instruction::Instruction;

#[test]
fn test_create_remittance_mint_stacked_extensions() {
    let mut svm = LiteSVM::new();
    let program_id = token22_remittance::id();
    let bytes = include_bytes!("../../../target/deploy/token22_remittance.so");
    svm.add_program(program_id, bytes).unwrap();

    let payer = Keypair::new();
    let mint = Keypair::new();
    let mint_authority = Keypair::new();
    let freeze_authority = Keypair::new();
    let close_authority = Keypair::new();
    let transfer_fee_config_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();
    let metadata_pointer_authority = Keypair::new();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let transfer_fee_basis_points = 250; // 2.5%
    let maximum_fee = 50_000_000; // 50 tokens
    let decimals = 6;

    let accounts = token22_remittance::accounts::CreateRemittanceMint {
        payer: payer.pubkey(),
        mint: mint.pubkey(),
        mint_authority: mint_authority.pubkey(),
        freeze_authority: freeze_authority.pubkey(),
        close_authority: close_authority.pubkey(),
        transfer_fee_config_authority: transfer_fee_config_authority.pubkey(),
        withdraw_withheld_authority: withdraw_withheld_authority.pubkey(),
        metadata_pointer_authority: metadata_pointer_authority.pubkey(),
        token_2022_program: anchor_spl::token_2022::ID,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let args = token22_remittance::instruction::CreateRemittanceMint {
        transfer_fee_basis_points,
        maximum_fee,
        decimals,
    };

    let ix = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: args.data(),
    };

    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[&payer, &mint], msg, blockhash);

    let tx_res = svm.send_transaction(tx);
    assert!(tx_res.is_ok(), "Transaction failed: {:?}", tx_res.err());

    // Task 3 verification: Read mint state exclusively through StateWithExtensions
    let mint_acc = svm.get_account(&mint.pubkey()).expect("Mint account not found");
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_acc.data).expect("Failed to unpack StateWithExtensions");

    // Verify Mint base state
    assert_eq!(mint_state.base.decimals, decimals);
    assert_eq!(mint_state.base.mint_authority.unwrap(), mint_authority.pubkey());
    assert_eq!(mint_state.base.freeze_authority.unwrap(), freeze_authority.pubkey());
    assert!(mint_state.base.is_initialized);

    // Verify 1: TransferFeeConfig extension
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().expect("TransferFeeConfig extension missing");
    assert_eq!(
        Option::<Pubkey>::from(fee_config.transfer_fee_config_authority),
        Some(transfer_fee_config_authority.pubkey())
    );
    assert_eq!(
        Option::<Pubkey>::from(fee_config.withdraw_withheld_authority),
        Some(withdraw_withheld_authority.pubkey())
    );
    assert_eq!(u16::from(fee_config.newer_transfer_fee.transfer_fee_basis_points), transfer_fee_basis_points);
    assert_eq!(u64::from(fee_config.newer_transfer_fee.maximum_fee), maximum_fee);

    // Verify 2: MetadataPointer extension pointed at mint itself
    let meta_pointer = mint_state.get_extension::<MetadataPointer>().expect("MetadataPointer extension missing");
    assert_eq!(
        Option::<Pubkey>::from(meta_pointer.authority),
        Some(metadata_pointer_authority.pubkey())
    );
    assert_eq!(
        Option::<Pubkey>::from(meta_pointer.metadata_address),
        Some(mint.pubkey())
    );

    // Verify 3: DefaultAccountState extension (Frozen)
    let default_state = mint_state.get_extension::<DefaultAccountState>().expect("DefaultAccountState extension missing");
    assert_eq!(AccountState::try_from(default_state.state).unwrap(), AccountState::Frozen);

    // Verify 4: MintCloseAuthority extension
    let close_ext = mint_state.get_extension::<MintCloseAuthority>().expect("MintCloseAuthority extension missing");
    assert_eq!(
        Option::<Pubkey>::from(close_ext.close_authority),
        Some(close_authority.pubkey())
    );
}
