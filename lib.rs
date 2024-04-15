#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod mock_dai {
    use ink::storage::Mapping;

    /// Create storage for the mockDai ERC20 token contract
    #[ink(storage)]
    pub struct MockDai {
        /// stores the total supply as the equal value of all active tokens.
        total_supply: Balance, // rust type
        /// balances mapping to store individual user balances
        balances: Mapping<AccountId, Balance>, // mapping of an account (address) to a balance
        /// mapping of all token amount allowances for this token
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    /// Transfer event to be fired when a token transfer occurs between users
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        amount: Balance,
    }

    /// Error specifications and handling
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Trigger if the balance of the caller account cannot fulfill a request
        InsufficientBalance,
        InsufficientAllowance,
    }

    /// Token result type specification
    pub type Result<T> = core::result::Result<T, Error>;

    impl MockDai {
        /// Let's create the mockDai token with an initial supply
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::default();
            let caller = Self::env().caller();
            let allowances = Mapping::default();

            // mint total supply to caller e.g rex
            balances.insert(caller, &total_supply);

            // fire the transfer event from the address(0) to address(rex) just like the EIP-20 specifies it
            Self::env().emit_event(Transfer {
                from: None,          // address(0)
                to: Some(caller),    // address(rex)
                value: total_supply, // e.g 1 mil DAI tokens
            });

            // mutate state variables of DAI tokens such as set the total supply to 1 million, balances of everybody where for example rex's DAI tokens is now 1 million and the allowances each account has setup for an operator for example in this case, no allowances -> all zeroed out
            Self {
                total_supply,
                balances,
                allowances,
            }
        }

        /// A public function also called a message that can be called on instantiated contracts.
        /// This returns the total supply of tokens for mockDai.
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /// Simply returns the token balance of a specified `account`
        #[ink(message)]
        pub fn balance_of(&self, account: AccountId) -> Balance {
            self.balances.get(account).unwrap_or_default()
        }

        /// Simply transfers mockDai tokens from caller to the receiving address `to`
        pub fn transfer(&mut self, to: AccountId, amount: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.transfer_from_to(&sender, &to, amount)
        }

        /// Approve spender to spend owner's tokens
        pub fn approve(&mut self, spender: AccountId, amount: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), &amount);

            self.env().emit_event(Approval {
                owner,
                spender,
                amount,
            });
            Ok(())
        }

        /// Allowance function to figure out the allowances of an address as allocated by an owner
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            // if there is an allowance, it should return the allowance otherwise the default will kick in which is 0 -> that is why we use the `unwrap_or_default` method on this get method for allowance
            self.allowances.get((owner, spender)).unwrap_or_default()
        }

        /// Similar TransferFrom in Solidity to allow the calling third-party or address to take tokens of the specified `from` account supposing they've already been approved for it
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, amount: Balance) -> Result<()> {
            let msg_sender = self.env().caller();
            let allowance = self.allowance(from, msg_sender);

            if allowance < amount {
                return Err(Error::InsufficientAllowance);
            }

            self.transfer_from_to(&from, &to, amount)?;
            self.allowances.insert((from, msg_sender), &(allowance - amount));

            Ok(())
        }
        

        /// Private function to handle the logic of tranfers
        fn transfer_from_to(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            amount: Balance,
        ) -> Result<()> {
            let sender_balance = self.balance_of(*from);

            if sender_balance < amount {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(sender_balance - amount));
            let to_balance = self.balance_of(*to);
            self.balances.insert(to, &(to_balance + amount));
            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;     

        /// We test if the default constructor does its job.
        #[ink::test]
        fn constructor_works() {
            let mock_dai = MockDai::new(1_000_000);
            assert_eq!(mock_dai.total_supply(), 1_000_000);
        }

        /// We write a test to simulate transfers and then return balances of Bob and Alice
        #[ink::test]
        fn balance_of_returns_correct_values() {
            // deploy an instance of MockDai token
            let mut mock_dai = MockDai::new(1_000_000);
            // make some mock accounts as we would with `makeAddr` in foundry equivalent
            
            // @note keep in mind that when we make mock addresses like we do below, the address derived from Account::from([1; 32]) is just the same as we do in a foundry test where address(this) is the calling contract during testing. so, to create actual accounts where the msg.sender isn't the deployer contract/address, we just skip making an address from 1.
            // this contract/deployer/msg.sender = AccountId::from([1; 32]);
            let bob = AccountId::from([2; 32]);
            let alice = AccountId::from([3; 32]);

            // check that before the DAI transfer, bob doesn't have any token balance
            assert_eq!(mock_dai.balance_of(bob), 0);

            // transfer some DAI to bob
            mock_dai.transfer(bob, 500).unwrap(); // we unwrap to return the actual value of the transfer transaction a.k.a result

            // check that bob recieved 500 DAI
            assert_eq!(mock_dai.balance_of(bob), 500);

            // check that during the transfer to bob, ALice didn't mistakenly get DAI tokens
            assert_eq!(mock_dai.balance_of(alice), 0);
        }

        #[ink::test]
        fn do_an_approval_check() {
            // @note if we were only making a read from the contract we can lose the `mut` key like below
            let mock_dai = MockDai::new(1_000_000);

            let bob = AccountId::from([2; 32]);
            let alice = AccountId::from([3; 32]);

            // make sure that there is no current allowances from bob to alice
            assert_eq!(mock_dai.allowance(bob, alice), 0);
        }
    }
}
