use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{Sysvar, rent::Rent},
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = CounterInstruction::unpack(instruction_data)?;

    match instruction {
        CounterInstruction::InitializeCounter { initial_value } => {
            process_initialize_counter(program_id, accounts, initial_value)?;
        }
        CounterInstruction::IncrementCounter => process_increment_counter(program_id, accounts)?,
    }

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    InitializeCounter { initial_value: u64 },
    IncrementCounter,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CounterAccount {
    count: u64,
}

impl CounterInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match variant {
            0 => {
                let initial_value = u64::from_le_bytes(
                    rest.try_into()
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                );

                Ok(Self::InitializeCounter { initial_value })
            }
            1 => Ok(Self::IncrementCounter),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

pub fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_space = 8;

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    invoke(
        &system_instruction::create_account(
            payer_account.key,
            counter_account.key,
            required_lamports,
            account_space as u64,
            program_id,
        ),
        &[
            payer_account.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
    )?;

    let counter_data = CounterAccount {
        count: initial_value,
    };

    let mut account_data = &mut counter_account.data.borrow_mut()[..];

    counter_data.serialize(&mut account_data)?;

    msg!("Counter initialized with value {} ", initial_value);

    Ok(())
}

pub fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;

    // verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut data = counter_account.data.borrow_mut();

    // deserialize the account data into out CounterAccount struct
    let mut counter_data: CounterAccount = CounterAccount::try_from_slice(&data)?;

    counter_data.count = counter_data
        .count
        .checked_add(1)
        .ok_or(ProgramError::InvalidAccountData)?;

    // serialize the updated counter data back into the account
    counter_data.serialize(&mut &mut data[..])?;

    msg!("Counter incremented to : {}", counter_data.count);
    Ok(())
}

#[cfg(test)]
mod test {
    use std::vec;

    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
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
