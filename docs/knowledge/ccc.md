# CCC (CKBers' Codebase) Knowledge

CCC is a TypeScript/JS SDK for the Nervos CKB blockchain (`@ckb-ccc/core`, `@ckb-ccc/shell`).

## 1. Construct, Sign (blake2b-secp256k1), and Send Transaction

### Setup

```typescript
import { ccc } from "@ckb-ccc/shell";

const client = new ccc.ClientPublicTestnet(); // or ClientPublicMainnet
const signer = new ccc.SignerCkbPrivateKey(client, "0xYOUR_32_BYTE_PRIVATE_KEY");
```

### Minimal Transfer

```typescript
const { script: receiverLock } = await ccc.Address.fromString("ckt1qyq...", client);

const tx = ccc.Transaction.from({
  outputs: [{ capacity: ccc.fixedPointFrom(100), lock: receiverLock }],
});

await tx.completeInputsByCapacity(signer);  // auto-collect inputs to cover output capacity
await tx.completeFeeBy(signer);             // auto-calc fee, create change output

const txHash = await signer.sendTransaction(tx);  // sign + send
await client.waitTransaction(txHash, 4);
```

### Signing Internals (blake2b-secp256k1)

1. For each unique lock script among inputs, a dummy 65-byte zero witness is set (sighash-all).
2. Cell deps for the secp256k1 script are added via `tx.addCellDepsOfKnownScripts(client, KnownScript.Secp256k1Blake160)`.
3. Signing hash: `blake2b(tx_hash || witness_0_len || witness_0 || ... || witness_n_len || witness_n)`.
4. The signing hash is signed with secp256k1 → DER-encoded 65 bytes (r 32 || s 32 || recovery 1).
5. The signature is placed into `witnessArgs.lock`.

### Transaction Builder Methods

| Method | Purpose |
|--------|---------|
| `tx.completeInputsByCapacity(signer, tweak?)` | Collect inputs to cover outputs + tweak |
| `tx.completeFeeBy(signer, feeRate?)` | Calculate fee, add more inputs if needed, create change output |
| `tx.completeFeeChangeToLock(signer, lock, feeRate?)` | Same but explicit change lock |
| `tx.addCellDepsOfKnownScripts(client, ...KnownScript)` | Add deps for known scripts |
| `tx.completeInputsAll(signer)` | Collect ALL cells as inputs |
| `tx.completeInputsByUdt(signer, type, tweak?)` | Collect UDT inputs by balance |
| `signer.sendTransaction(tx)` | Sign + send |
| `signer.signTransaction(tx)` | Sign only (returns signed tx) |
| `client.sendTransaction(tx)` | Send a pre-signed tx |

### Address → Lock Script

```typescript
const { script: lock } = await ccc.Address.fromString(
  "ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqd...", client
);
// lock.codeHash, lock.hashType, lock.args
```

### Known Script Identities

```typescript
ccc.KnownScript.Secp256k1Blake160   // default lock
ccc.KnownScript.NervosDao           // DAO
ccc.KnownScript.XUdt                // xUDT
ccc.KnownScript.SighashAll          // sighash-all
ccc.KnownScript.AnyoneCanPay        // anyone-can-pay
```

## 2. Find and Search DAO Deposits

### DAO Type Script

```typescript
const daoType = await ccc.Script.fromKnownScript(
  client, ccc.KnownScript.NervosDao, "0x"
);
// Result: codeHash, hashType="type", args="0x"
```

### Create a DAO Deposit

```typescript
const depositTx = ccc.Transaction.from({
  outputs: [{
    // capacity >= ~82 CKB (occupied size of a DAO cell)
    capacity: ccc.fixedPointFrom(200),
    lock,           // your lock (e.g., Secp256k1Blake160)
    type: daoType,  // NervosDao type script
  }],
  outputsData: ["0x0000000000000000"],  // 8 bytes of zero = deposit phase
});

await depositTx.addCellDepsOfKnownScripts(client, ccc.KnownScript.NervosDao);
await depositTx.completeInputsByCapacity(signer);
await depositTx.completeFeeBy(signer);
const txHash = await signer.sendTransaction(depositTx);
```

### Search DAO Cells

```typescript
// Method 1: findCellsByType
for await (const cell of client.findCellsByType(daoType)) {
  const isDeposited = await cell.isNervosDao(client, "deposited");
  const isWithdrew  = await cell.isNervosDao(client, "withdrew");
}

// Method 2: findCells with filters
for await (const cell of signer.findCells({
  script: daoType,              // type script to match
  scriptType: "type",           // match type (not lock)
  scriptSearchMode: "exact",    // exact or prefix
  scriptLenRange: [33, 34],     // DAO type script byte length
  outputDataLenRange: [8, 9],   // DAO output data byte length
})) {
  // check phase
  const isDeposited = await cell.isNervosDao(client, "deposited");
  if (isWithdrew) {
    const profit = await cell.getDaoProfit(client);
    console.log("profit:", profit);
  }
}
```

### DAO Utility Methods on `Cell`

| Method | Description |
|--------|-------------|
| `cell.isNervosDao(client, phase?)` | Check if DAO cell; optional phase: `"deposited"` or `"withdrew"` |
| `cell.getDaoProfit(client)` | Calculate confirmed DAO profit (withdrawal phase 2) |
| `cell.getNervosDaoInfo(client)` | Returns `{ depositHeader }` or `{ depositHeader, withdrawHeader }` |

### Standalone DAO Functions

```typescript
import { ccc } from "@ckb-ccc/core";

ccc.calcDaoProfit(capacity, depositHeader, withdrawHeader);
// = capacity * withdrawHeader.dao.ar / depositHeader.dao.ar - capacity

ccc.calcDaoClaimEpoch(depositHeader, withdrawHeader);
// Returns the epoch when withdrawal becomes claimable
```

### DAO Phases (by outputData)

| Phase | outputData |
|-------|-----------|
| Deposit | `0x0000000000000000` (8 bytes of zero) |
| Withdrawal (Phase 1) | 8-byte LE block number where deposit was made |
| Claim | N/A (cell is consumed, CKB + interest returned) |

## 3. Molecule Operations

Molecule is CKB's canonical binary serialization format. CCC provides a complete TypeScript codec layer.

### Basic Codecs

```typescript
import { mol } from "@ckb-ccc/core";

// Fixed-size integers (little-endian)
mol.Uint8, mol.Uint16, mol.Uint32, mol.Uint64, mol.Uint128, mol.Uint256, mol.Uint512

// Big-endian variants
mol.Uint32BE, mol.Uint64BE

// Fixed-size byte arrays (returned as Hex strings)
mol.Byte4, mol.Byte8, mol.Byte16, mol.Byte32

// Dynamic types
mol.Bytes       // 32-bit LE length prefix + data
mol.Bool        // 1 byte
mol.String      // UTF-8 string (Bytes-encoded)

// Optional wrappers
mol.Uint32Opt, mol.Byte32Opt, mol.BytesOpt
```

### Composite Codecs

```typescript
// Struct — fixed-size, ALL fields must be fixed-size
const OutPointCodec = mol.struct({
  txHash: mol.Byte32,
  index:  mol.Uint32,
});

// Table — dynamic-size, dynamic fields (Bytes, vectors, etc.) must come LAST
const ScriptCodec = mol.table({
  codeHash: mol.Byte32,   // fixed
  hashType: mol.Byte,     // fixed
  args:     mol.Bytes,    // dynamic — must be last
});

// Vector — auto-chooses FixVec or DynVec depending on inner codec
const OutPointVec = mol.vector(OutPointCodec);

// Option — nullable; result is `T | undefined`
const ScriptOpt = mol.option(ScriptCodec);

// Union — tagged union, tag determined by field order
const WitnessArgsCodec = mol.union({
  Bytes: mol.Bytes,
  WitnessArgs: WitnessArgsTable,
});
```

### Using with Classes (`@mol.codec` + `Entity.Base`)

Decorator-based approach used for all CKB data structures in CCC:

```typescript
@mol.codec(
  mol.table({
    codeHash: mol.Byte32,
    hashType: HashTypeCodec,
    args: mol.Bytes,
  })
)
export class Script extends mol.Entity.Base<ScriptLike, Script>() {
  constructor(public codeHash: Hex, public hashType: HashType, public args: Hex) {
    super();
  }
}
```

A class annotated with `@mol.codec` automatically gets:

| Method | Description |
|--------|-------------|
| `instance.toBytes()` | Serialize to bytes |
| `instance.clone()` | Deep copy via encode/decode |
| `instance.eq(other)` | Byte-level equality |
| `instance.hash()` | blake2b hash of encoded bytes |
| `Script.encode(inst)` | Static encode |
| `Script.decode(bytes)` | Static decode (returns plain object) |
| `Script.fromBytes(bytes)` | Static decode + constructor call |

### Custom Codecs

```typescript
// 1-byte codec mapping strings to bytes
const HashTypeCodec = mol.Codec.from({
  byteLength: 1,
  encode(val) {
    if (val === "type")  return "0x00";
    if (val === "data")  return "0x01";
    if (val === "data1") return "0x02";
    if (val === "data2") return "0x03";
    throw new Error(`Unknown hash type: ${val}`);
  },
  decode(b) {
    const v = Number(b.getUint8(0));
    return ["type","data","data1","data2"][v];
  },
});
```

### mapIn / mapOut

Transform data on encoding/decoding (e.g., convenience constructors):

```typescript
// Since uses Uint64 with a map to convert structured Since <-> bigint
const SinceCodec = mol.Uint64
  .mapIn(encodable => Since.from(encodable).toNum())
  .mapOut(num => Since.from(num));

// ScriptOpt maps undefined <-> empty option
const ScriptOptCodec = mol.option(ScriptCodec)
  .mapIn(s => s ?? mol.none())      // undefined -> none
  .mapOut(s => s ?? undefined);     // none -> undefined
```

### Encode / Decode Patterns

```typescript
// Encode a value to hex bytes
const bytes = ScriptCodec.encode({ codeHash: "...", hashType: "type", args: "0x" });
// => "0x..."

// Encode a class instance
const script = new Script("0x9bd7...", "type", "0x");
const bytes = script.toBytes();

// Decode bytes to plain object
const obj = ScriptCodec.decode("0x...");

// Decode bytes to class instance
const script = Script.fromBytes("0x...");
```

### Vector Packing

Vectors hold an item count (4 bytes LE) at the start, followed by items. The encoder automatically computes the count:

```typescript
const vec = mol.vector(mol.Byte32);
const encoded = vec.encode(["0xab...", "0xcd...", "0xef..."]);
// => 0x03000000 + ab... + cd... + ef...
```

### Bytes Packing

```typescript
mol.Bytes.encode("0xaabbcc");
// => 0x03000000aabbcc  (4-byte LE length + data)

mol.Bytes.decode("0x03000000aabbcc");
// => "0xaabbcc"
```
