use anchor_lang::{
    system_program::ID as SYSTEM_PROGRAM_ID,
    InstructionData, ToAccountMetas,
};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        default_account_state::DefaultAccountState,
        metadata_pointer::MetadataPointer,
        mint_close_authority::MintCloseAuthority,
        transfer_fee::{TransferFeeAmount, TransferFeeConfig},
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::{Account, AccountState, Mint},
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

#[test]
fn test_transfer_with_fee_dynamic_calculation() {
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

    let alice = Keypair::new();
    let bob = Keypair::new();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&alice.pubkey(), 1_000_000_000).unwrap();

    let transfer_fee_basis_points = 250; // 2.5%
    let maximum_fee = 50_000_000; // 50 tokens
    let decimals = 6;

    // 1. Create remittance mint
    let create_mint_accounts = token22_remittance::accounts::CreateRemittanceMint {
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

    let create_mint_ix = Instruction {
        program_id,
        accounts: create_mint_accounts.to_account_metas(None),
        data: token22_remittance::instruction::CreateRemittanceMint {
            transfer_fee_basis_points,
            maximum_fee,
            decimals,
        }.data(),
    };

    let msg = Message::new(&[create_mint_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &mint], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    // 2. Derive ATAs for Alice and Bob
    let alice_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(
        &alice.pubkey(),
        &mint.pubkey(),
        &anchor_spl::token_2022::ID,
    );
    let bob_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(
        &bob.pubkey(),
        &mint.pubkey(),
        &anchor_spl::token_2022::ID,
    );

    let create_alice_ata_ix = anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &alice.pubkey(),
        &mint.pubkey(),
        &anchor_spl::token_2022::ID,
    );
    let create_bob_ata_ix = anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &bob.pubkey(),
        &mint.pubkey(),
        &anchor_spl::token_2022::ID,
    );

    let msg = Message::new(&[create_alice_ata_ix, create_bob_ata_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    // Verify both ATAs start in Frozen state due to DefaultAccountState::Frozen
    let alice_acc_pre = svm.get_account(&alice_ata).unwrap();
    let alice_state_pre = StateWithExtensions::<Account>::unpack(&alice_acc_pre.data).unwrap();
    assert_eq!(alice_state_pre.base.state, AccountState::Frozen);

    // 3. Thaw Alice and Bob via freeze_authority
    let thaw_alice_ix = anchor_spl::token_2022::spl_token_2022::instruction::thaw_account(
        &anchor_spl::token_2022::ID,
        &alice_ata,
        &mint.pubkey(),
        &freeze_authority.pubkey(),
        &[],
    ).unwrap();
    let thaw_bob_ix = anchor_spl::token_2022::spl_token_2022::instruction::thaw_account(
        &anchor_spl::token_2022::ID,
        &bob_ata,
        &mint.pubkey(),
        &freeze_authority.pubkey(),
        &[],
    ).unwrap();

    let msg = Message::new(&[thaw_alice_ix, thaw_bob_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &freeze_authority], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    // 4. Mint 1,000 tokens (1,000,000,000 raw) to Alice
    let initial_mint_amount = 1_000_000_000u64;
    let mint_to_ix = anchor_spl::token_2022::spl_token_2022::instruction::mint_to(
        &anchor_spl::token_2022::ID,
        &mint.pubkey(),
        &alice_ata,
        &mint_authority.pubkey(),
        &[],
        initial_mint_amount,
    ).unwrap();

    let msg = Message::new(&[mint_to_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &mint_authority], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();

    // 5. Transfer 100 tokens with fee from Alice to Bob
    let transfer_amount = 100_000_000u64;
    // Expected fee: 100_000_000 * 250 / 10_000 = 2_500_000 (2.5 tokens)
    let expected_fee = 2_500_000u64;

    let transfer_accounts = token22_remittance::accounts::TransferRemittanceWithFee {
        authority: alice.pubkey(),
        from: alice_ata,
        mint: mint.pubkey(),
        to: bob_ata,
        token_2022_program: anchor_spl::token_2022::ID,
    };

    let transfer_ix = Instruction {
        program_id,
        accounts: transfer_accounts.to_account_metas(None),
        data: token22_remittance::instruction::TransferWithFee {
            amount: transfer_amount,
        }.data(),
    };

    let msg = Message::new(&[transfer_ix], Some(&alice.pubkey()));
    let tx = Transaction::new(&[&alice], msg, svm.latest_blockhash());
    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Transfer failed: {:?}", res.err());

    // 6. Verify balances and withheld fee via StateWithExtensions
    let alice_acc_post = svm.get_account(&alice_ata).unwrap();
    let alice_state_post = StateWithExtensions::<Account>::unpack(&alice_acc_post.data).unwrap();
    assert_eq!(alice_state_post.base.amount, initial_mint_amount - transfer_amount);

    let bob_acc_post = svm.get_account(&bob_ata).unwrap();
    let bob_state_post = StateWithExtensions::<Account>::unpack(&bob_acc_post.data).unwrap();
    // Bob receives transferred amount (97_500_000 net spendable tokens)
    assert_eq!(bob_state_post.base.amount, transfer_amount - expected_fee);

    // Withheld transfer fee is recorded on Bob's account under TransferFeeAmount extension
    let bob_fee_amount = bob_state_post.get_extension::<TransferFeeAmount>().expect("TransferFeeAmount extension missing");
    assert_eq!(u64::from(bob_fee_amount.withheld_amount), expected_fee);
}

