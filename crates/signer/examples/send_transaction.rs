//! Example of signing and sending a transaction using a Keycard.

use std::{str::FromStr, sync::Arc};

use alloy::{
    network::TransactionBuilder,
    primitives::{U256, address},
    providers::{Provider, ProviderBuilder},
    rpc::types::transaction::TransactionRequest,
};
use nexum_keycard::{Keycard, PairingInfo, PcscDeviceManager};
use nexum_keycard_signer::{DerivationPath, KeycardSigner};

use eyre::Result;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Establish transport to the Keycard
    let device_manager = PcscDeviceManager::new()?;
    let readers = device_manager.list_readers()?;
    let reader = readers.first().expect("No readers found");
    let transport = device_manager.open_reader(reader.name())?;

    // Your known credentials and derivation path
    let pin = "123456".to_string();
    let pairing_info = PairingInfo {
        index: 0,       // Your pairing index
        key: [0u8; 32], // Replace with your actual pairing key
    };
    let path = DerivationPath::from_str("m/44'/60'/0'/0/0")?;

    // Instantiate the Keycard instance with known credentials
    let keycard = Keycard::with_known_credentials(transport, pin, pairing_info)?;

    // Wrap the keycard in Arc<Mutex<_>> for thread-safe access
    let keycard = Arc::new(Mutex::new(keycard));

    // Create the signer
    let signer = KeycardSigner::new(keycard.clone(), path).await?;

    // Create a provider with the wallet.
    let rpc_url = "https://reth-ethereum.ithaca.xyz/rpc".parse()?;
    let provider = ProviderBuilder::new().wallet(signer).on_http(rpc_url);

    // Build a transaction to send 100 wei from Alice to Vitalik
    // The `from` field is automatically filled to the first signer's address (Alice).
    let vitalik = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    let tx = TransactionRequest::default()
        .with_to(vitalik)
        .with_value(U256::from(100));

    // Send the transaction and wait for inclusion with 3 confirmations.
    let tx_hash = provider
        .send_transaction(tx)
        .await?
        .with_required_confirmations(3)
        .watch()
        .await?;

    println!("Sent transaction hash: {tx_hash}");

    Ok(())
}
