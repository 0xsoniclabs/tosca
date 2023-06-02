#include "vm/evmzero/interpreter.h"

#include <bit>
#include <cstdio>

#include "vm/evmzero/opcodes.h"

namespace tosca::evmzero {

const char* ToString(RunState state) {
  switch (state) {
    case RunState::kRunning:
      return "Running";
    case RunState::kDone:
      return "Done";
    case RunState::kInvalid:
      return "Invalid";
    case RunState::kErrorOpcode:
      return "ErrorOpcode";
    case RunState::kErrorGas:
      return "ErrorGas";
    case RunState::kErrorStack:
      return "ErrorStack";
    case RunState::kErrorJump:
      return "ErrorJump";
    case RunState::kErrorCall:
      return "ErrorCall";
    case RunState::kErrorCreate:
      return "ErrorCreate";
  }
  return "UNKNOWN_STATE";
}

std::ostream& operator<<(std::ostream& out, RunState state) { return out << ToString(state); }

InterpreterResult Interpret(const InterpreterArgs& args) {
  internal::Context ctx;
  ctx.code.assign(args.code.begin(), args.code.end());

  internal::RunInterpreter(ctx);

  if (ctx.state != RunState::kDone) {
    ctx.gas = 0;
  }

  return {
      .state = ctx.state,
      .remaining_gas = ctx.gas,
      .return_data = ctx.return_data,
  };
}

///////////////////////////////////////////////////////////

namespace op {

using internal::Context;

static void stop(Context& ctx) noexcept { ctx.state = RunState::kDone; }

static void add(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a + b);
  ctx.pc++;
}

static void mul(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(5)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a * b);
  ctx.pc++;
}

static void sub(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a - b);
  ctx.pc++;
}

static void div(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(5)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  if (b == 0)
    ctx.stack.Push(0);
  else
    ctx.stack.Push(a / b);
  ctx.pc++;
}

static void sdiv(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(5)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  if (b == 0)
    ctx.stack.Push(0);
  else
    ctx.stack.Push(intx::sdivrem(a, b).quot);
  ctx.pc++;
}

static void mod(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(5)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  if (b == 0)
    ctx.stack.Push(0);
  else
    ctx.stack.Push(a % b);
  ctx.pc++;
}

static void smod(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(5)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  if (b == 0)
    ctx.stack.Push(0);
  else
    ctx.stack.Push(intx::sdivrem(a, b).rem);
  ctx.pc++;
}

static void addmod(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(3)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(8)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  uint256_t N = ctx.stack.Pop();
  if (N == 0)
    ctx.stack.Push(0);
  else
    ctx.stack.Push(intx::addmod(a, b, N));
  ctx.pc++;
}

static void mulmod(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(3)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(8)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  uint256_t N = ctx.stack.Pop();
  if (N == 0)
    ctx.stack.Push(0);
  else
    ctx.stack.Push(intx::mulmod(a, b, N));
  ctx.pc++;
}

static void exp(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(10)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t exponent = ctx.stack.Pop();
  if (!ctx.ApplyGasCost(50 * intx::count_significant_bytes(exponent))) [[unlikely]]
    return;
  ctx.stack.Push(intx::exp(a, exponent));
  ctx.pc++;
}

static void signextend(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(5)) [[unlikely]]
    return;

  uint8_t leading_byte_index = static_cast<uint8_t>(ctx.stack.Pop());
  if (leading_byte_index > 31) {
    leading_byte_index = 31;
  }

  uint256_t value = ctx.stack.Pop();

  bool is_negative = ToByteArrayLe(value)[leading_byte_index] & 0b1000'0000;
  if (is_negative) {
    auto mask = kUint256Max << (8 * (leading_byte_index + 1));
    ctx.stack.Push(mask | value);
  } else {
    auto mask = kUint256Max >> (8 * (31 - leading_byte_index));
    ctx.stack.Push(mask & value);
  }

  ctx.pc++;
}

static void lt(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a < b ? 1 : 0);
  ctx.pc++;
}

static void gt(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a > b ? 1 : 0);
  ctx.pc++;
}

static void slt(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(intx::slt(a, b) ? 1 : 0);
  ctx.pc++;
}

static void sgt(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(intx::slt(b, a) ? 1 : 0);
  ctx.pc++;
}

static void eq(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a == b ? 1 : 0);
  ctx.pc++;
}

static void iszero(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t val = ctx.stack.Pop();
  ctx.stack.Push(val == 0);
  ctx.pc++;
}

static void bit_and(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a & b);
  ctx.pc++;
}

static void bit_or(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a | b);
  ctx.pc++;
}

static void bit_xor(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  uint256_t b = ctx.stack.Pop();
  ctx.stack.Push(a ^ b);
  ctx.pc++;
}

static void bit_not(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t a = ctx.stack.Pop();
  ctx.stack.Push(~a);
  ctx.pc++;
}

static void byte(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t offset = ctx.stack.Pop();
  uint256_t x = ctx.stack.Pop();
  if (offset < 32) {
    // Offset starts at most significant byte.
    ctx.stack.Push(ToByteArrayLe(x)[31 - static_cast<uint8_t>(offset)]);
  } else {
    ctx.stack.Push(0);
  }
  ctx.pc++;
}

static void shl(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t shift = ctx.stack.Pop();
  uint256_t value = ctx.stack.Pop();
  ctx.stack.Push(value << shift);
  ctx.pc++;
}

static void shr(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t shift = ctx.stack.Pop();
  uint256_t value = ctx.stack.Pop();
  ctx.stack.Push(value >> shift);
  ctx.pc++;
}

static void sar(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  uint256_t shift = ctx.stack.Pop();
  uint256_t value = ctx.stack.Pop();
  const bool is_negative = ToByteArrayLe(value)[31] & 0b1000'0000;

  if (shift > 31) {
    shift = 31;
  }

  value >>= shift;

  if (is_negative) {
    value |= (kUint256Max << (31 - shift));
  }

  ctx.stack.Push(value);
  ctx.pc++;
}

static void sha3(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(30)) [[unlikely]]
    return;

  const uint64_t offset = static_cast<uint64_t>(ctx.stack.Pop());
  const uint64_t size = static_cast<uint64_t>(ctx.stack.Pop());

  const uint64_t minimum_word_size = (size + 31) / 32;
  if (!ctx.ApplyGasCost(6 * minimum_word_size + ctx.MemoryExpansionCost(offset + size))) [[unlikely]]
    return;

  std::vector<uint8_t> buffer(size);
  ctx.memory.WriteTo(buffer, offset);

  auto hash = ethash::keccak256(buffer.data(), buffer.size());
  ctx.stack.Push(ToUint256(hash));
  ctx.pc++;
}

static void pop(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(2)) [[unlikely]]
    return;
  ctx.stack.Pop();
  ctx.pc++;
}

static void mload(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  const uint64_t offset = static_cast<uint64_t>(ctx.stack.Pop());
  if (!ctx.ApplyGasCost(ctx.MemoryExpansionCost(offset + 32))) [[unlikely]]
    return;

  uint256_t value;
  ctx.memory.WriteTo({ToBytes(value), 32}, offset);

  if constexpr (std::endian::native == std::endian::little) {
    value = intx::bswap(value);
  }

  ctx.stack.Push(value);
  ctx.pc++;
}

static void mstore(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  const uint64_t offset = static_cast<uint64_t>(ctx.stack.Pop());
  uint256_t value = ctx.stack.Pop();
  if (!ctx.ApplyGasCost(ctx.MemoryExpansionCost(offset + 32))) [[unlikely]]
    return;

  if constexpr (std::endian::native == std::endian::little) {
    value = intx::bswap(value);
  }

  ctx.memory.ReadFrom({ToBytes(value), 32}, offset);
  ctx.pc++;
}

static void mstore8(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  const uint64_t offset = static_cast<uint64_t>(ctx.stack.Pop());
  const uint8_t value = static_cast<uint8_t>(ctx.stack.Pop());
  if (!ctx.ApplyGasCost(ctx.MemoryExpansionCost(offset + 1))) [[unlikely]]
    return;

  ctx.memory.ReadFrom({&value, 1}, offset);
  ctx.pc++;
}

static void jump(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(8)) [[unlikely]]
    return;
  uint64_t counter = static_cast<uint64_t>(ctx.stack.Pop());
  if (!ctx.CheckJumpDest(counter)) [[unlikely]]
    return;
  ctx.pc = counter;
}

static void jumpi(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(2)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(10)) [[unlikely]]
    return;
  uint64_t counter = static_cast<uint64_t>(ctx.stack.Pop());
  uint64_t b = static_cast<uint64_t>(ctx.stack.Pop());
  if (b != 0) {
    if (!ctx.CheckJumpDest(counter)) [[unlikely]]
      return;
    ctx.pc = counter;
  } else {
    ctx.pc++;
  }
}

static void pc(Context& ctx) noexcept {
  if (!ctx.CheckStackOverflow(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(2)) [[unlikely]]
    return;
  ctx.stack.Push(ctx.pc);
  ctx.pc++;
}

static void msize(Context& ctx) noexcept {
  if (!ctx.CheckStackOverflow(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(2)) [[unlikely]]
    return;
  ctx.stack.Push(ctx.memory.GetSize());
  ctx.pc++;
}

static void gas(Context& ctx) noexcept {
  if (!ctx.CheckStackOverflow(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(2)) [[unlikely]]
    return;
  ctx.stack.Push(ctx.gas);
  ctx.pc++;
}

static void jumpdest(Context& ctx) noexcept {
  if (!ctx.ApplyGasCost(1)) [[unlikely]]
    return;
  ctx.pc++;
}

template <uint64_t N>
static void push(Context& ctx) noexcept {
  if (!ctx.CheckStackOverflow(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;

  // If their aren't enough values in the code, we exit without doing the push
  // as the interpreter would stop with the next iteration anyway.
  if (ctx.code.size() < ctx.pc + 1 + N) [[unlikely]] {
    ctx.pc += 1 + N;
    ctx.state = RunState::kDone;
    return;
  }

  uint256_t value = 0;
  for (uint64_t i = 1; i <= N; ++i) {
    value |= static_cast<uint256_t>(ctx.code[ctx.pc + i]) << (N - i) * 8;
  }
  ctx.stack.Push(value);
  ctx.pc += 1 + N;
}

template <uint64_t N>
static void dup(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(N)) [[unlikely]]
    return;
  if (!ctx.CheckStackOverflow(1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  ctx.stack.Push(ctx.stack[N - 1]);
  ctx.pc++;
}

template <uint64_t N>
static void swap(Context& ctx) noexcept {
  if (!ctx.CheckStackAvailable(N + 1)) [[unlikely]]
    return;
  if (!ctx.ApplyGasCost(3)) [[unlikely]]
    return;
  std::swap(ctx.stack[0], ctx.stack[N]);
  ctx.pc++;
}

}  // namespace op

///////////////////////////////////////////////////////////

namespace internal {

bool Context::CheckStackAvailable(uint64_t elements_needed) noexcept {
  if (stack.GetSize() < elements_needed) [[unlikely]] {
    state = RunState::kErrorStack;
    return false;
  } else {
    return true;
  }
}

bool Context::CheckStackOverflow(uint64_t slots_needed) noexcept {
  if (stack.GetMaxSize() - stack.GetSize() < slots_needed) [[unlikely]] {
    state = RunState::kErrorStack;
    return false;
  } else {
    return true;
  }
}

bool Context::CheckJumpDest(uint64_t index) noexcept {
  FillValidJumpTargetsUpTo(index);
  if (index >= code.size() || !valid_jump_targets[index]) [[unlikely]] {
    state = RunState::kErrorJump;
    return false;
  } else {
    return true;
  }
}

void Context::FillValidJumpTargetsUpTo(uint64_t index) noexcept {
  if (index < valid_jump_targets.size()) [[likely]] {
    return;
  }

  if (index >= code.size()) [[unlikely]] {
    assert(false);
    return;
  }

  const uint64_t old_size = valid_jump_targets.size();
  valid_jump_targets.resize(index + 1);

  uint64_t cur = old_size;
  while (cur <= index) {
    const auto instruction = code[cur];

    if (op::PUSH1 <= instruction && instruction <= op::PUSH32) {
      // Skip PUSH and arguments
      cur += instruction - op::PUSH1 + 2;
    } else {
      valid_jump_targets[cur] = instruction == op::JUMPDEST;
      cur++;
    }
  }
}

uint64_t Context::MemoryExpansionCost(uint64_t new_size) noexcept {
  if (new_size <= memory.GetSize()) {
    return 0;
  }

  auto calc_memory_cost = [](uint64_t size) {
    uint64_t memory_size_word = (size + 31) / 32;
    return (memory_size_word * memory_size_word) / 512 + (3 * memory_size_word);
  };

  return calc_memory_cost(new_size) - calc_memory_cost(memory.GetSize());
}

bool Context::ApplyGasCost(uint64_t gas_cost) noexcept {
  if (gas < gas_cost) [[unlikely]] {
    state = RunState::kErrorGas;
    return false;
  }

  gas -= gas_cost;

  return true;
}

void RunInterpreter(Context& ctx) {
  while (ctx.state == RunState::kRunning) {
    if (ctx.pc >= ctx.code.size()) [[unlikely]] {
      ctx.state = RunState::kErrorOpcode;
      break;
    }

    switch (ctx.code[ctx.pc]) {
      // clang-format off
      case op::STOP: op::stop(ctx); break;

      case op::ADD: op::add(ctx); break;
      case op::MUL: op::mul(ctx); break;
      case op::SUB: op::sub(ctx); break;
      case op::DIV: op::div(ctx); break;
      case op::SDIV: op::sdiv(ctx); break;
      case op::MOD: op::mod(ctx); break;
      case op::SMOD: op::smod(ctx); break;
      case op::ADDMOD: op::addmod(ctx); break;
      case op::MULMOD: op::mulmod(ctx); break;
      case op::EXP: op::exp(ctx); break;
      case op::SIGNEXTEND: op::signextend(ctx); break;
      case op::LT: op::lt(ctx); break;
      case op::GT: op::gt(ctx); break;
      case op::SLT: op::slt(ctx); break;
      case op::SGT: op::sgt(ctx); break;
      case op::EQ: op::eq(ctx); break;
      case op::ISZERO: op::iszero(ctx); break;
      case op::AND: op::bit_and(ctx); break;
      case op::OR: op::bit_or(ctx); break;
      case op::XOR: op::bit_xor(ctx); break;
      case op::NOT: op::bit_not(ctx); break;
      case op::BYTE: op::byte(ctx); break;
      case op::SHL: op::shl(ctx); break;
      case op::SHR: op::shr(ctx); break;
      case op::SAR: op::sar(ctx); break;
      case op::SHA3: op::sha3(ctx); break;
      /*
      case op::ADDRESS: op::address(ctx); break;
      case op::BALANCE: op::balance(ctx); break;
      case op::ORIGIN: op::origin(ctx); break;
      case op::CALLER: op::caller(ctx); break;
      case op::CALLVALUE: op::callvalue(ctx); break;
      case op::CALLDATALOAD: op::calldataload(ctx); break;
      case op::CALLDATASIZE: op::calldatasize(ctx); break;
      case op::CALLDATACOPY: op::calldatacopy(ctx); break;
      case op::CODESIZE: op::codesize(ctx); break;
      case op::CODECOPY: op::codecopy(ctx); break;
      case op::GASPRICE: op::gasprice(ctx); break;
      case op::EXTCODESIZE: op::extcodesize(ctx); break;
      case op::EXTCODECOPY: op::extcodecopy(ctx); break;
      case op::RETURNDATASIZE: op::returndatasize(ctx); break;
      case op::RETURNDATACOPY: op::returndatacopy(ctx); break;
      case op::EXTCODEHASH: op::extcodehash(ctx); break;
      case op::BLOCKHASH: op::blockhash(ctx); break;
      case op::COINBASE: op::coinbase(ctx); break;
      case op::TIMESTAMP: op::timestamp(ctx); break;
      case op::NUMBER: op::number(ctx); break;
      case op::DIFFICULTY: op::prevrandao(ctx); break; // intentional
      case op::GASLIMIT: op::gaslimit(ctx); break;
      case op::CHAINID: op::chainid(ctx); break;
      case op::SELFBALANCE: op::selfbalance(ctx); break;
      case op::BASEFEE: op::basefee(ctx); break;
      */

      case op::POP: op::pop(ctx); break;
      case op::MLOAD: op::mload(ctx); break;
      case op::MSTORE: op::mstore(ctx); break;
      case op::MSTORE8: op::mstore8(ctx); break;
      /*
      case op::SLOAD: op::sload(ctx); break;
      case op::SSTORE: op::sstore(ctx); break;
      */

      case op::JUMP: op::jump(ctx); break;
      case op::JUMPI: op::jumpi(ctx); break;
      case op::PC: op::pc(ctx); break;
      case op::MSIZE: op::msize(ctx); break;
      case op::GAS: op::gas(ctx); break;
      case op::JUMPDEST: op::jumpdest(ctx); break;

      case op::PUSH1: op::push<1>(ctx); break;
      case op::PUSH2: op::push<2>(ctx); break;
      case op::PUSH3: op::push<3>(ctx); break;
      case op::PUSH4: op::push<4>(ctx); break;
      case op::PUSH5: op::push<5>(ctx); break;
      case op::PUSH6: op::push<6>(ctx); break;
      case op::PUSH7: op::push<7>(ctx); break;
      case op::PUSH8: op::push<8>(ctx); break;
      case op::PUSH9: op::push<9>(ctx); break;
      case op::PUSH10: op::push<10>(ctx); break;
      case op::PUSH11: op::push<11>(ctx); break;
      case op::PUSH12: op::push<12>(ctx); break;
      case op::PUSH13: op::push<13>(ctx); break;
      case op::PUSH14: op::push<14>(ctx); break;
      case op::PUSH15: op::push<15>(ctx); break;
      case op::PUSH16: op::push<16>(ctx); break;
      case op::PUSH17: op::push<17>(ctx); break;
      case op::PUSH18: op::push<18>(ctx); break;
      case op::PUSH19: op::push<19>(ctx); break;
      case op::PUSH20: op::push<20>(ctx); break;
      case op::PUSH21: op::push<21>(ctx); break;
      case op::PUSH22: op::push<22>(ctx); break;
      case op::PUSH23: op::push<23>(ctx); break;
      case op::PUSH24: op::push<24>(ctx); break;
      case op::PUSH25: op::push<25>(ctx); break;
      case op::PUSH26: op::push<26>(ctx); break;
      case op::PUSH27: op::push<27>(ctx); break;
      case op::PUSH28: op::push<28>(ctx); break;
      case op::PUSH29: op::push<29>(ctx); break;
      case op::PUSH30: op::push<30>(ctx); break;
      case op::PUSH31: op::push<31>(ctx); break;
      case op::PUSH32: op::push<32>(ctx); break;

      case op::DUP1: op::dup<1>(ctx); break;
      case op::DUP2: op::dup<2>(ctx); break;
      case op::DUP3: op::dup<3>(ctx); break;
      case op::DUP4: op::dup<4>(ctx); break;
      case op::DUP5: op::dup<5>(ctx); break;
      case op::DUP6: op::dup<6>(ctx); break;
      case op::DUP7: op::dup<7>(ctx); break;
      case op::DUP8: op::dup<8>(ctx); break;
      case op::DUP9: op::dup<9>(ctx); break;
      case op::DUP10: op::dup<10>(ctx); break;
      case op::DUP11: op::dup<11>(ctx); break;
      case op::DUP12: op::dup<12>(ctx); break;
      case op::DUP13: op::dup<13>(ctx); break;
      case op::DUP14: op::dup<14>(ctx); break;
      case op::DUP15: op::dup<15>(ctx); break;
      case op::DUP16: op::dup<16>(ctx); break;

      case op::SWAP1: op::swap<1>(ctx); break;
      case op::SWAP2: op::swap<2>(ctx); break;
      case op::SWAP3: op::swap<3>(ctx); break;
      case op::SWAP4: op::swap<4>(ctx); break;
      case op::SWAP5: op::swap<5>(ctx); break;
      case op::SWAP6: op::swap<6>(ctx); break;
      case op::SWAP7: op::swap<7>(ctx); break;
      case op::SWAP8: op::swap<8>(ctx); break;
      case op::SWAP9: op::swap<9>(ctx); break;
      case op::SWAP10: op::swap<10>(ctx); break;
      case op::SWAP11: op::swap<11>(ctx); break;
      case op::SWAP12: op::swap<12>(ctx); break;
      case op::SWAP13: op::swap<13>(ctx); break;
      case op::SWAP14: op::swap<14>(ctx); break;
      case op::SWAP15: op::swap<15>(ctx); break;
      case op::SWAP16: op::swap<16>(ctx); break;

      /*
      case op::LOG0: op::log<0>(ctx); break;
      case op::LOG1: op::log<1>(ctx); break;
      case op::LOG2: op::log<2>(ctx); break;
      case op::LOG3: op::log<3>(ctx); break;
      case op::LOG4: op::log<4>(ctx); break;

      case op::CREATE: op::create_impl<op::CREATE>(ctx); break;
      case op::CREATE2: op::create_impl<op::CREATE2>(ctx); break;

      case op::RETURN: op::return_op(ctx); break;
      case op::REVERT: op::return_op(ctx); break; // TODO

      case op::CALL: op::call_impl<op::CALL>(ctx); break;
      case op::CALLCODE: op::call_impl<op::CALLCODE>(ctx); break;
      case op::DELEGATECALL: op::call_impl<op::DELEGATECALL>(ctx); break;
      case op::STATICCALL: op::call_impl<op::STATICCALL>(ctx); break;

      case op::INVALID: op::invalid(ctx); break;
      case op::SELFDESTRUCT: op::selfdestruct(ctx); break;
      */
        // clang-format on
      default:
        ctx.state = RunState::kErrorOpcode;
    }
  }
}

}  // namespace internal

}  // namespace tosca::evmzero
