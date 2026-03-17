use anchor_lang::{prelude::*, solana_program::program::invoke};
use anchor_spl::token_interface::Mint;
use solana_instruction::Instruction;
use spl_tlv_account_resolution::state::ExtraAccountMetaList;

use crate::{
    errors::HookProgramError,
    state::{
        deposit_hook_permissionless, get_deposit_hook_extra_account_metas_address,
        DepositHookInstruction,
    },
};

#[derive(Accounts)]
pub struct DepositHook<'info> {
    // This should be the vault authority
    pub signer: Signer<'info>,
    pub share_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: This is the extra metas
    pub extra_metas: AccountInfo<'info>,
    /// CHECK: This is downstream protocol
    pub protocol: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> DepositHook<'info> {
    pub fn invoke_deposit(
        &self,
        program_id: &Pubkey,
        additional_accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        let mut instruction = deposit_hook_permissionless(
            &self.protocol.key(),
            self.signer.key,
            &self.share_mint.key(),
        );

        let validation_pubkey =
            get_deposit_hook_extra_account_metas_address(&self.share_mint.key(), program_id);

        let mut cpi_account_infos = vec![
            self.signer.to_account_info(),
            self.share_mint.to_account_info(),
            self.protocol.to_account_info(),
        ];

        if self.extra_metas.key() == validation_pubkey {
            instruction
                .accounts
                .push(AccountMeta::new_readonly(validation_pubkey, false));
            let validation_info = self.extra_metas.to_account_info();
            cpi_account_infos.push(validation_info.clone());
            ExtraAccountMetaList::add_to_cpi_instruction::<DepositHookInstruction>(
                &mut instruction,
                &mut cpi_account_infos,
                &validation_info.try_borrow_data()?,
                additional_accounts,
            )?;
        } else {
            return Err(HookProgramError::InvalidAccountData.into());
        }

        cpi_account_infos.remove(2);
        cpi_account_infos.remove(2);
        instruction.accounts.remove(2);
        instruction.accounts.remove(2);
        instruction
            .accounts
            .push(AccountMeta::new_readonly(self.system_program.key(), false));
        cpi_account_infos.push(self.system_program.to_account_info());
        invoke(&instruction, &cpi_account_infos)?;
        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, DepositHook<'info>>) -> Result<()> {
    ctx.accounts.invoke_deposit(
        &pubkey!("ANXYYTDoEHooFjaN8M8pDHRj87d945Bj5QvAFGcpqakw"),
        ctx.remaining_accounts,
    )?;
    Ok(())
}
