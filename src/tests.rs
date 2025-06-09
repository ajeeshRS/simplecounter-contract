#[cfg(test)]
mod test {
    use std::vec;
    use crate::{processor::process_instruction, state::CounterAccount};
    use borsh::BorshDeserialize;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction}, pubkey::Pubkey, signature::{Keypair, Signer}, system_program, transaction::Transaction
    };

    #[tokio::test]
    async fn test_counter_program() {
        let program_id = Pubkey::new_unique();

        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "counter_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        let counter_keypair = Keypair::new();
        let initial_value: u64 = 1;

        // step 1
        println!("Testing counter initialization..");

        // create initialization instruction
        let mut init_instruction_data = vec![0]; //0 = initialize instruction with initial value

        init_instruction_data.extend_from_slice(&initial_value.to_le_bytes());

        let initialize_instruction = Instruction::new_with_bytes(
            program_id,
            &init_instruction_data,
            vec![
                AccountMeta::new(counter_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let mut transaction =
            Transaction::new_with_payer(&[initialize_instruction], Some(&payer.pubkey()));

        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);

        banks_client.process_transaction(transaction).await.unwrap();

        // check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 1);
            println!(
                "Counter initialized successfullt with value : {}",
                counter.count
            );
        }

        // step 2 : increment the counter
        let increment_instruction = Instruction::new_with_bytes(
            program_id,
            &[1],
            vec![AccountMeta::new(counter_keypair.pubkey(), true)],
        );

        let mut transaction =
            Transaction::new_with_payer(&[increment_instruction], Some(&payer.pubkey()));

        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // check account data

        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("failed to deserialize counte data");
            assert_eq!(counter.count, 2);
            println!("Counter incremented successfullu to : {}", counter.count);
        }
    }
}