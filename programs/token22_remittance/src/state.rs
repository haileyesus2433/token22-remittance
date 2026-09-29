use anchor_lang::prelude::*;
use spl_token_2022_interface::{
    extension::{
        confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
        confidential_transfer_fee::ConfidentialTransferFeeConfig,
        default_account_state::DefaultAccountState,
        metadata_pointer::MetadataPointer,
        mint_close_authority::MintCloseAuthority,
        permanent_delegate::PermanentDelegate,
        transfer_fee::{TransferFeeAmount, TransferFeeConfig},
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::{Account, Mint},
};
use crate::error::RemittanceError;

pub struct StateReader;

impl StateReader {
    /// Unpack mint account using StateWithExtensions, strictly avoiding raw unpack
    pub fn unpack_mint<'a>(data: &'a [u8]) -> Result<StateWithExtensions<'a, Mint>> {
        StateWithExtensions::<Mint>::unpack(data).map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Unpack token account using StateWithExtensions, strictly avoiding raw unpack
    pub fn unpack_account<'a>(data: &'a [u8]) -> Result<StateWithExtensions<'a, Account>> {
        StateWithExtensions::<Account>::unpack(data).map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract TransferFeeConfig from mint
    pub fn get_transfer_fee_config<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a TransferFeeConfig> {
        mint_state
            .get_extension::<TransferFeeConfig>()
            .map_err(|_| RemittanceError::MissingTransferFeeConfig.into())
    }

    /// Extract DefaultAccountState from mint
    pub fn get_default_account_state<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a DefaultAccountState> {
        mint_state
            .get_extension::<DefaultAccountState>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract MetadataPointer from mint
    pub fn get_metadata_pointer<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a MetadataPointer> {
        mint_state
            .get_extension::<MetadataPointer>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract MintCloseAuthority from mint
    pub fn get_mint_close_authority<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a MintCloseAuthority> {
        mint_state
            .get_extension::<MintCloseAuthority>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract PermanentDelegate from mint
    pub fn get_permanent_delegate<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a PermanentDelegate> {
        mint_state
            .get_extension::<PermanentDelegate>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract ConfidentialTransferMint from mint
    pub fn get_confidential_transfer_mint<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a ConfidentialTransferMint> {
        mint_state
            .get_extension::<ConfidentialTransferMint>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract ConfidentialTransferFeeConfig from mint
    pub fn get_confidential_transfer_fee_config<'a>(
        mint_state: &'a StateWithExtensions<'a, Mint>,
    ) -> Result<&'a ConfidentialTransferFeeConfig> {
        mint_state
            .get_extension::<ConfidentialTransferFeeConfig>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract TransferFeeAmount from token account
    pub fn get_transfer_fee_amount<'a>(
        account_state: &'a StateWithExtensions<'a, Account>,
    ) -> Result<&'a TransferFeeAmount> {
        account_state
            .get_extension::<TransferFeeAmount>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }

    /// Extract ConfidentialTransferAccount from token account
    pub fn get_confidential_transfer_account<'a>(
        account_state: &'a StateWithExtensions<'a, Account>,
    ) -> Result<&'a ConfidentialTransferAccount> {
        account_state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| RemittanceError::InvalidExtension.into())
    }
}
