mod assert;
mod edition_marker;
mod external_price;
mod master_edition_v2;
mod metadata;
mod vault;

pub use assert::*;
pub use edition_marker::EditionMarker;
pub use external_price::ExternalPrice;
pub use master_edition_v2::MasterEditionV2;
pub use metadata::{assert_collection_size, Metadata};
pub use mpl_token_metadata::instruction;
use mpl_token_metadata::state::CollectionDetails;
use solana_program_test::*;
use solana_sdk::{
    account::Account, program_pack::Pack, pubkey::Pubkey, signature::Signer,
    signer::keypair::Keypair, system_instruction, transaction::Transaction,
};
use spl_token::state::Mint;
pub use vault::Vault;

pub const DEFAULT_COLLECTION_DETAILS: Option<CollectionDetails> =
    Some(CollectionDetails::V1 { size: 0 });

pub fn program_test() -> ProgramTest {
    ProgramTest::new("mpl_token_metadata", mpl_token_metadata::id(), None)
}

pub async fn get_account(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Account {
    context
        .banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn get_mint(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Mint {
    let account = get_account(context, pubkey).await;
    Mint::unpack(&account.data).unwrap()
}

pub async fn airdrop(
    context: &mut ProgramTestContext,
    receiver: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            receiver,
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
    Ok(())
}

pub async fn burn(
    context: &mut ProgramTestContext,
    metadata: Pubkey,
    owner: &Keypair,
    mint: Pubkey,
    token: Pubkey,
    edition: Pubkey,
    collection_metadata: Option<Pubkey>,
    spl_token: Pubkey,
) -> Result<(), BanksClientError> {
    let tx = Transaction::new_signed_with_payer(
        &[instruction::burn_nft(
            mpl_token_metadata::ID,
            metadata,
            owner.pubkey(),
            mint,
            token,
            edition,
            spl_token,
            collection_metadata,
        )],
        Some(&owner.pubkey()),
        &[owner],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn mint_tokens(
    context: &mut ProgramTestContext,
    mint: &Pubkey,
    account: &Pubkey,
    amount: u64,
    owner: &Pubkey,
    additional_signer: Option<&Keypair>,
    spl_token: &Pubkey,
) -> Result<(), BanksClientError> {
    let mut signing_keypairs = vec![&context.payer];
    if let Some(signer) = additional_signer {
        signing_keypairs.push(signer);
    }

    let mint_to = if *spl_token == spl_token::id() {
        spl_token::instruction::mint_to
    } else {
        spl_token_2022::instruction::mint_to
    };

    let tx = Transaction::new_signed_with_payer(
        &[
            mint_to(spl_token, mint, account, owner, &[], amount)
                .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &signing_keypairs,
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
    spl_token: &Pubkey,
) -> Result<(), BanksClientError> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let initialize_account = if *spl_token == spl_token::id() {
        spl_token::instruction::initialize_account
    } else {
        spl_token_2022::instruction::initialize_account
    };

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                spl_token,
            ),
            initialize_account(
                spl_token,
                &account.pubkey(),
                mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    manager: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    decimals: u8,
) -> Result<(), BanksClientError> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                manager,
                freeze_authority,
                decimals,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}


// maybe use spl-token-client once it's been published w/ token-2022
pub async fn create_mint_with_close_authority(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    manager: &Pubkey,
    decimals: u8,
) -> Result<(), BanksClientError> {
    use spl_token_2022::extension::ExtensionType;
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = ExtensionType::get_account_len::<
        spl_token_2022::state::Mint>(&[ExtensionType::MintCloseAuthority]);

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &spl_token_2022::id(),
            ),
            spl_token_2022::instruction::initialize_mint_close_authority(
                &spl_token_2022::id(),
                &mint.pubkey(),
                Some(manager),
            )
            .unwrap(),
            spl_token_2022::instruction::initialize_mint(
                &spl_token_2022::id(),
                &mint.pubkey(),
                manager,
                Some(manager),
                decimals,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}


