# Antigravity Execution & Reasoning History (`agy_history.md`)

This log records the complete trajectory of user prompts, agent thinking, design decisions, and implementation steps during the development of `trx-processor`.

---

## Phase 1: Core Types & Store Implementation

### User Request
> "Please can you help me to finish @[src/transactions.rs] we just need to fix the trx to have concret types that's impl TryFrom<&[u8}> and that's should be placed on @[src/types.rs] the methods of store should handle types to and from that's impl TryFrom<&[u8]>"

### Agent Thinking & Analysis
1. **Orphan Rule & Type Design**:
   - Primitive types like `u64` do not implement `TryFrom<&[u8]>` in Rust standard library because endianness is unspecified, and Rust orphan rules prevent `impl TryFrom<&[u8]> for u64` in external crates.
   - Concrete wrapper types were needed in `src/types.rs`:
     - `TransactionRecord` (15-byte fixed representation for `client_id`, `tx_id`, `amount`, `status`).
     - `Amount` (wrapper around `u64` for fixed-point integer values scaled by `10000`).
     - `LockStatus` (1-byte wrapper around `bool`).
2. **Store API Design**:
   - `StoreItem` was updated to wrap `Vec<u8>`.
   - `Store` methods were made generic over types `T` satisfying higher-ranked trait bounds `for<'a> TryFrom<&'a [u8]> + Into<Vec<u8>>`:
     - `get<T>(&self, key: Key) -> Option<T>`
     - `put<T>(&mut self, key: Key, value: T)`
     - `upsert<T, F>(&mut self, key: Key, default: T, f: F)`
3. **Transaction Engine Fixes**:
   - Fixed slice boundary bugs in key generator helper functions (`available_key`, `held_key`, `total_key`, `locked_key`, `trx_key`).
   - Implemented full processing logic for `Deposit`, `Withdrawal`, `Dispute`, `Resolve`, and `ChargerBack`.
   - Implemented CSV export formatter (`as_csv`).

---

## Phase 2: Dual Parser Architecture

### User Request
> "on the parser you should use csv crate to parser csv using serde and on corasick we will use aho-corasick algo that's is provided by crate aho-corasick, this will help us to face two performance approaches."

### Agent Thinking & Analysis
1. **Dependencies**:
   - Added `csv = "1.3"` and `serde = { version = "1.0", features = ["derive"] }` to `Cargo.toml`.
2. **Implementation**:
   - `src/parser/csv.rs`: Built `parse_csv` using `csv::ReaderBuilder` and `serde::Deserialize` into a flexible `Record` struct.
   - `src/parser/corasick.rs`: Initial implementation of Aho-Corasick parser.
   - `src/parser/mod.rs`: Updated `Parser::parse` to return `Box<dyn Iterator<Item = CsvItem>>` for dynamic dispatch between `--engine=csv` and `--engine=corasick`.

---

## Phase 3: High-Performance 32KB Zero-Copy Aho-Corasick Optimization

### User Request
> "the implementation of corasick is wrong, if you try to read lines then you will read the file two times and makes things a little worse, please implement the buff with 32kb size, divide it by two and the if you don't match a pattern until the new line character then you will read more data, in this case we will read only one time and make no copies of data with trim and blabla, don't use the entire word on pattern just two first character that will improve the speed because AVX2."

### Agent Thinking & Analysis
1. **Root Cause of Performance Bottleneck in Initial Parser**:
   - Using `lines()` read the file line-by-line, allocated `String` objects, trimmed whitespace, and performed secondary splitting—creating high allocation pressure and double-scanning memory.
2. **Redesigning `src/parser/corasick.rs`**:
   - **32KB Buffer Window**: Allocated a static 32KB buffer (`vec![0u8; 32768]`).
   - **Single-Pass Reading**: Read chunks directly into the available space in the buffer.
   - **Sliding Window Shift**: Found the last `\n` in the buffer (`process_len`). Unparsed tail bytes beyond the last `\n` were shifted to index 0 using `copy_within(process_len..buf_len, 0)` so incomplete lines across chunk boundaries were never split or lost.
   - **2-Character AVX2 Prefix Matching**: Pattern set configured as `&["de", "wi", "di", "re", "ch"]` to trigger SIMD/AVX2 multi-byte search routines in `aho-corasick`.
   - **Zero-Allocation Byte Parsing**: Written custom integer (`parse_u16`, `parse_u32`) and fixed-point amount (`parse_amount_bytes`) parsers operating directly on `&[u8]` slices without string allocation or `trim()`.

### Empirical Verification & Benchmark
- Benchmark on **208,252 synthetic transactions**:
  - `csv` engine: ~916 ms
  - `corasick` 32KB zero-copy engine: ~535 ms (**~1.7x speedup**)
  - Diff test verified 100% exact numerical match across all output client balances.
- All 8 unit tests passed cleanly.
