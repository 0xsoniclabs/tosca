use evmc_vm::{AccessStatus, Address, Revision};

use crate::{
    types::{ExecutionContextTrait, FailStatus, u256},
    utils::word_size,
};

/// The gas refund accumulated during execution. It may become negative.
#[derive(Debug)]
pub struct GasRefund(i64);

impl GasRefund {
    /// Creates a new [`GasRefund`] with the given amount.
    pub fn new(gas: i64) -> Self {
        Self(gas)
    }

    /// Returns the refund as `i64`.
    pub fn as_i64(&self) -> i64 {
        self.0
    }

    /// Adds `gas`, which may be negative, to the refund, wrapping on overflow.
    #[inline(always)]
    pub fn add(&mut self, gas: i64) {
        self.0 = self.0.wrapping_add(gas);
    }
}

/// The gas left for execution.
// Invariant: gas <= i64::MAX
#[derive(Debug)]
pub struct Gas(u64);

impl PartialEq<u64> for Gas {
    fn eq(&self, other: &u64) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<u64> for Gas {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(other))
    }
}

impl Gas {
    /// Creates a new [`Gas`] with the given amount, clamping negative amounts to zero.
    pub fn new(gas: i64) -> Self {
        Self(gas.max(0).cast_unsigned())
    }

    /// Returns the gas left as `u64`.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Returns the gas left as `i64`. The gas left never exceeds `i64::MAX`, so the cast never
    /// produces a negative value.
    pub fn as_i64(&self) -> i64 {
        self.0.cast_signed()
    }

    /// Adds `gas`, which may be negative, to the gas left. Fails with [`FailStatus::OutOfGas`] if
    /// the result is negative or overflows `i64`.
    #[inline(always)]
    pub fn add(&mut self, gas: i64) -> Result<(), FailStatus> {
        let (gas, overflow) = self.0.cast_signed().overflowing_add(gas);
        if gas < 0 || overflow {
            std::hint::cold_path();
            return Err(FailStatus::OutOfGas);
        }
        self.0 = gas.cast_unsigned();
        Ok(())
    }

    /// Consumes `gas` or fails with [`FailStatus::OutOfGas`] if not enough gas is left.
    #[inline(always)]
    pub fn consume(&mut self, gas: u64) -> Result<(), FailStatus> {
        if self.0 < gas {
            std::hint::cold_path();
            return Err(FailStatus::OutOfGas);
        }
        self.0 -= gas;
        Ok(())
    }

    /// Consumes the cost of transferring `value`, which is only charged if `value` is non-zero.
    #[inline(always)]
    pub fn consume_positive_value_cost(&mut self, value: &u256) -> Result<(), FailStatus> {
        if *value != u256::ZERO {
            self.consume(9_000)?;
        }
        Ok(())
    }

    /// Consumes the cost of transferring `value` to `addr`, which is only charged if `value` is
    /// non-zero and `addr` does not exist.
    #[inline(always)]
    pub fn consume_value_to_empty_account_cost(
        &mut self,
        value: &u256,
        addr: &Address,
        context: &mut dyn ExecutionContextTrait,
    ) -> Result<(), FailStatus> {
        if *value != u256::ZERO && !context.account_exists(addr) {
            self.consume(25_000)?;
        }
        Ok(())
    }

    /// Consumes the cost of accessing `addr`, which depends on whether it is cold or warm. Access
    /// is free before Berlin.
    #[inline(always)]
    pub fn consume_address_access_cost(
        &mut self,
        addr: &Address,
        revision: Revision,
        context: &mut dyn ExecutionContextTrait,
    ) -> Result<(), FailStatus> {
        if revision < Revision::EVMC_BERLIN {
            return Ok(());
        }
        if context.access_account(addr) == AccessStatus::EVMC_ACCESS_COLD {
            self.consume(2_600)
        } else {
            self.consume(100)
        }
    }

    /// Consumes the access cost of the delegation target if the code of `addr` is a delegation
    /// designator (`0xef0100 || address`). Delegation does not exist before Prague.
    #[inline(always)]
    pub fn consume_delegate_resolution_cost(
        &mut self,
        addr: &Address,
        revision: Revision,
        context: &mut dyn ExecutionContextTrait,
    ) -> Result<(), FailStatus> {
        if revision < Revision::EVMC_PRAGUE {
            return Ok(());
        }
        if context.get_code_size(addr) == 23 {
            let mut code = [0; 23];
            context.copy_code(addr, 0, &mut code);
            if let [0xef, 0x01, 0x00, delegation @ ..] = code {
                let delegation_addr = Address { bytes: delegation };
                self.consume_address_access_cost(&delegation_addr, revision, context)?;
            }
        }
        Ok(())
    }

    /// Consumes the cost of copying `len` bytes, which is 3 per started word.
    #[inline(always)]
    pub fn consume_copy_cost(&mut self, len: u64) -> Result<(), FailStatus> {
        let cost = word_size(len)? * 3; // does not overflow because word_size divides by 32
        self.consume(cost)
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate;
    use rstest::rstest;

    use super::*;
    use crate::types::MockExecutionContextTrait;

    #[test]
    fn gas_refund_new_wraps_value() {
        assert_eq!(GasRefund::new(-1).0, -1);
    }

    #[test]
    fn gas_refund_as_i64_returns_internal_value() {
        assert_eq!(GasRefund(-1).as_i64(), -1);
    }

    #[rstest]
    #[case::positive(1, 2, 3)]
    #[case::negative(1, -2, -1)]
    #[case::wraps_on_overflow(i64::MAX, 1, i64::MIN)]
    #[case::wraps_on_underflow(i64::MIN, -1, i64::MAX)]
    fn gas_refund_add_adds_with_wrapping(
        #[case] refund: i64,
        #[case] gas: i64,
        #[case] expected: i64,
    ) {
        let mut refund = GasRefund::new(refund);
        refund.add(gas);
        assert_eq!(refund.as_i64(), expected);
    }

    #[test]
    fn gas_compares_to_u64() {
        let gas = Gas::new(1);
        assert!(gas == 1);
        assert!(gas != 2);
        assert!(gas < 2);
        assert!(gas > 0);
    }

    #[rstest]
    #[case::positive(1, 1)]
    #[case::max(i64::MAX, i64::MAX.cast_unsigned())]
    #[case::negative_is_clamped_to_zero(-1, 0)]
    fn gas_new_clamps_negative_values_to_zero(#[case] gas: i64, #[case] expected: u64) {
        assert_eq!(Gas::new(gas).as_u64(), expected);
    }

    #[test]
    fn gas_as_u64_returns_internal_value() {
        assert_eq!(Gas(1).as_u64(), 1);
    }

    #[test]
    fn gas_as_i64_returns_non_negative_value_up_to_i64_max() {
        assert_eq!(Gas(i64::MAX.cast_unsigned()).as_i64(), i64::MAX);
    }

    #[rstest]
    #[case::positive(1, 2, Ok(()), 3)]
    #[case::negative(2, -1, Ok(()), 1)]
    #[case::negative_to_zero(1, -1, Ok(()), 0)]
    #[case::negative_below_zero(1, -2, Err(FailStatus::OutOfGas), 1)]
    #[case::overflow(i64::MAX, 1, Err(FailStatus::OutOfGas), i64::MAX.cast_unsigned())]
    fn gas_add_adds_gas_and_fails_if_the_result_is_negative_or_overflows(
        #[case] gas: i64,
        #[case] add: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let mut gas = Gas::new(gas);
        assert_eq!(gas.add(add), expected);
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::nothing(1, 0, Ok(()), 1)]
    #[case::some(2, 1, Ok(()), 1)]
    #[case::all(1, 1, Ok(()), 0)]
    #[case::out_of_gas(1, 2, Err(FailStatus::OutOfGas), 1)]
    fn consume_subtracts_gas_and_fails_if_not_enough_gas_is_left(
        #[case] gas: i64,
        #[case] cost: u64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let mut gas = Gas::new(gas);
        assert_eq!(gas.consume(cost), expected);
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::zero_value(u256::ZERO, 0, Ok(()), 0)]
    #[case::positive_value_exact_gas_left(u256::ONE, 9_000, Ok(()), 0)]
    #[case::positive_value_more_gas_left(u256::ONE, 9_001, Ok(()), 1)]
    #[case::positive_non_one_value(u256::from(3u8), 9_001, Ok(()), 1)]
    #[case::out_of_gas(u256::ONE, 8_999, Err(FailStatus::OutOfGas), 8_999)]
    fn consume_positive_value_cost_charges_non_zero_values_and_fails_if_out_of_gas(
        #[case] value: u256,
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let mut gas = Gas::new(gas);
        assert_eq!(gas.consume_positive_value_cost(&value), expected);
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::zero_value(u256::ZERO, None, 0, Ok(()), 0)]
    #[case::existing_account(u256::ONE, Some(true), 0, Ok(()), 0)]
    #[case::non_existing_account_exact_gas_left(u256::ONE, Some(false), 25_000, Ok(()), 0)]
    #[case::non_existing_account_more_gas_left(u256::ONE, Some(false), 25_001, Ok(()), 1)]
    #[case::out_of_gas(u256::ONE, Some(false), 24_999, Err(FailStatus::OutOfGas), 24_999)]
    fn consume_value_to_empty_account_cost_charges_non_zero_values_to_non_existing_accounts_and_fails_if_out_of_gas(
        #[case] value: u256,
        #[case] exists: Option<bool>,
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let addr = Address::from(u256::ONE);
        let mut context = MockExecutionContextTrait::new();
        if let Some(exists) = exists {
            context
                .expect_account_exists()
                .times(1)
                .with(predicate::eq(addr))
                .return_const(exists);
        }

        let mut gas = Gas::new(gas);
        assert_eq!(
            gas.consume_value_to_empty_account_cost(&value, &addr, &mut context),
            expected
        );
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::before_berlin(Revision::EVMC_ISTANBUL, None, 0, Ok(()), 0)]
    #[case::cold(Revision::EVMC_BERLIN, Some(AccessStatus::EVMC_ACCESS_COLD), 2_600, Ok(()), 0)]
    #[case::warm(Revision::EVMC_BERLIN, Some(AccessStatus::EVMC_ACCESS_WARM), 100, Ok(()), 0)]
    #[case::out_of_gas(
        Revision::EVMC_BERLIN,
        Some(AccessStatus::EVMC_ACCESS_COLD),
        2_599,
        Err(FailStatus::OutOfGas),
        2_599
    )]
    fn consume_address_access_cost_charges_cold_and_warm_access_since_berlin_and_fails_if_out_of_gas(
        #[case] revision: Revision,
        #[case] access_status: Option<AccessStatus>,
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let addr = Address::from(u256::ONE);
        let mut context = MockExecutionContextTrait::new();
        if let Some(access_status) = access_status {
            context
                .expect_access_account()
                .times(1)
                .with(predicate::eq(addr))
                .return_const(access_status);
        }

        let mut gas = Gas::new(gas);
        assert_eq!(
            gas.consume_address_access_cost(&addr, revision, &mut context),
            expected
        );
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::before_prague(Revision::EVMC_CANCUN, None, None, 0, Ok(()), 0)]
    #[case::code_size_is_not_23(Revision::EVMC_PRAGUE, Some(vec![0xef, 0x01, 0x00]), None, 0, Ok(()), 0)]
    #[case::no_delegation_prefix(Revision::EVMC_PRAGUE, Some(vec![0; 23]), None, 0, Ok(()), 0)]
    #[case::cold_delegation(
        Revision::EVMC_PRAGUE,
        Some([vec![0xef, 0x01, 0x00], vec![2; 20]].concat()),
        Some(AccessStatus::EVMC_ACCESS_COLD),
        2_600,
        Ok(()),
        0
    )]
    #[case::warm_delegation(
        Revision::EVMC_PRAGUE,
        Some([vec![0xef, 0x01, 0x00], vec![2; 20]].concat()),
        Some(AccessStatus::EVMC_ACCESS_WARM),
        100,
        Ok(()),
        0
    )]
    #[case::out_of_gas(
        Revision::EVMC_PRAGUE,
        Some([vec![0xef, 0x01, 0x00], vec![2; 20]].concat()),
        Some(AccessStatus::EVMC_ACCESS_COLD),
        2_599,
        Err(FailStatus::OutOfGas),
        2_599
    )]
    fn consume_delegate_resolution_cost_charges_delegation_target_access_since_prague_and_fails_if_out_of_gas(
        #[case] revision: Revision,
        #[case] code: Option<Vec<u8>>,
        #[case] access_status: Option<AccessStatus>,
        #[case] gas: i64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let addr = Address::from(u256::ONE);
        let mut context = MockExecutionContextTrait::new();
        if let Some(code) = code {
            context
                .expect_get_code_size()
                .times(1)
                .with(predicate::eq(addr))
                .return_const(code.len());
            if code.len() == 23 {
                context
                    .expect_copy_code()
                    .times(1)
                    .with(predicate::eq(addr), predicate::eq(0), predicate::always())
                    .returning(move |_, _, buffer| {
                        buffer.copy_from_slice(&code);
                        code.len()
                    });
            }
        }
        if let Some(access_status) = access_status {
            context
                .expect_access_account()
                .times(1)
                .with(predicate::eq(Address { bytes: [2; 20] }))
                .return_const(access_status);
        }

        let mut gas = Gas::new(gas);
        assert_eq!(
            gas.consume_delegate_resolution_cost(&addr, revision, &mut context),
            expected
        );
        assert_eq!(gas, gas_left);
    }

    #[rstest]
    #[case::zero_len(1, 0, Ok(()), 1)]
    #[case::rounds_up_to_a_word(3, 1, Ok(()), 0)]
    #[case::exact_word(3, 32, Ok(()), 0)]
    #[case::second_word(6, 33, Ok(()), 0)]
    #[case::out_of_gas(2, 1, Err(FailStatus::OutOfGas), 2)]
    #[case::word_count_overflows(2, u64::MAX, Err(FailStatus::OutOfGas), 2)]
    fn consume_copy_cost_charges_3_per_word_and_fails_if_out_of_gas(
        #[case] gas: i64,
        #[case] len: u64,
        #[case] expected: Result<(), FailStatus>,
        #[case] gas_left: u64,
    ) {
        let mut gas = Gas::new(gas);
        assert_eq!(gas.consume_copy_cost(len), expected);
        assert_eq!(gas, gas_left);
    }
}
