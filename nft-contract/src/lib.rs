use near_sdk::{near, require, AccountId};
use near_sdk_contract_tools::nft::*;

#[derive(Default, NonFungibleToken)]
#[near(contract_state)]
pub struct Contract {
    current_token_id: u64,
}

#[near]
impl Contract {
    #[private]
    pub fn update_metadata(&mut self, metadata: ContractMetadata) {
        self.set_contract_metadata(&metadata);
    }

    #[payable]
    pub fn mint_for(&mut self, receiver_id: AccountId, media_url: String) -> String {
        require!(
            near_sdk::env::predecessor_account_id() == "relay.intear.near",
            "Only the minter can mint for others"
        );
        Nep145Controller::deposit_to_storage_account(
            self,
            &receiver_id,
            near_sdk::env::attached_deposit().into(),
        )
        .expect("Deposit failed");
        let token_id = self.current_token_id.to_string();
        Nep177Controller::mint_with_metadata(
            self,
            &token_id,
            &receiver_id,
            &TokenMetadata {
                title: Some("NEARvember NFT".to_string()),
                description: Some("NEARvember NFT".to_string()),
                media: Some(media_url),
                ..Default::default()
            },
        )
        .expect("MintFor failed");
        self.current_token_id += 1;
        token_id
    }

    #[private]
    pub fn set_next_token_id(&mut self, token_id: u64) {
        self.current_token_id = token_id;
    }
}
