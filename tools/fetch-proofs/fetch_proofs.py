#!/usr/bin/env python3
"""
Fetch fulfilled proof requests from the Succinct Prover Network via gRPC.

Proto schema sourced from:
  ~/.cargo/registry/.../sp1-sdk-6.1.0/src/network/proto/auction/types.rs

Fee units (raw API values are in atto-PROVE, 1 PROVE = 10^18 atto-PROVE):
  - base_fee         raw atto-PROVE         → PROVE (/ 1e18)
  - price_per_bpgu   raw atto-PROVE per PGU → PROVE/bPGU (/ 1e9, i.e. / 1e18 × 1e9)
  - total_fee        base_fee + gas_used/1e9 * price_per_bpgu  (in PROVE)

Fields output per proof:
  - request_id       hex string
  - mode             proof mode (Core / Compressed / Plonk / Groth16)
  - sp1_version      version string (e.g. "sp1-v5.0.0")
  - gas_used         prover gas units consumed
  - time_taken       human-readable duration
  - price_per_bpgu   PROVE per billion PGU
  - base_fee         PROVE
  - total_fee        PROVE (estimated)

Usage:
    pip install httpx h2
    python3 fetch_proofs.py [--limit N] [--pages N] [--mode all|plonk|groth16|...]

Dependencies: httpx, h2  (pip install httpx h2)
"""

import argparse
import json
import re
import struct
import sys
from datetime import datetime, timezone

import httpx

GRPC_ENDPOINT = "https://rpc.mainnet.succinct.xyz"

PROOF_MODE = {
    0: "Unknown",
    1: "Core",
    2: "Compressed",
    3: "Plonk",
    4: "Groth16",
}

FULFILLMENT_STATUS = {
    0: "Unknown",
    1: "Requested",
    2: "Assigned",
    3: "Fulfilled",
    4: "Unfulfillable",
}

PROOF_MODE_NAME_TO_INT = {v.lower(): k for k, v in PROOF_MODE.items()}

_VERSION_RE = re.compile(r"sp1-v(\d+)(?:\.(\d+))?(?:\.(\d+))?")


def _version_tuple(version: str) -> tuple[int, ...] | None:
    """Parse 'sp1-vX.Y.Z' into (X, Y, Z). Returns None if unparseable."""
    m = _VERSION_RE.match(version)
    if not m:
        return None
    return tuple(int(x) for x in m.groups(default="0"))

# ---------------------------------------------------------------------------
# Minimal protobuf encoder
# ---------------------------------------------------------------------------

def _encode_varint(value: int) -> bytes:
    result = bytearray()
    while value > 0x7F:
        result.append(0x80 | (value & 0x7F))
        value >>= 7
    result.append(value & 0x7F)
    return bytes(result)


def _varint_field(field_number: int, value: int) -> bytes:
    tag = _encode_varint((field_number << 3) | 0)  # wire type 0 = varint
    return tag + _encode_varint(value)


def _grpc_frame(message_bytes: bytes) -> bytes:
    """Prepend the 5-byte gRPC frame: 1 byte compression=0, 4 bytes big-endian length."""
    return b"\x00" + struct.pack(">I", len(message_bytes)) + message_bytes


def _encode_get_filtered_requests(
    fulfillment_status: int = 3,  # FULFILLED
    limit: int = 100,
    page: int = 1,
) -> bytes:
    """
    Encode GetFilteredProofRequestsRequest.

    Field tags (from sp1-sdk-6.1.0 auction/types.rs):
      tag 2  = fulfillment_status (optional enum varint)
      tag 10 = limit (optional uint32 varint)
      tag 11 = page  (optional uint32 varint)
    """
    body = b""
    body += _varint_field(2, fulfillment_status)
    body += _varint_field(10, limit)
    body += _varint_field(11, page)
    return body


# ---------------------------------------------------------------------------
# Minimal protobuf decoder
# ---------------------------------------------------------------------------

class _Decoder:
    def __init__(self, data: bytes):
        self.data = data
        self.pos = 0

    def done(self) -> bool:
        return self.pos >= len(self.data)

    def _read_byte(self) -> int:
        b = self.data[self.pos]
        self.pos += 1
        return b

    def read_varint(self) -> int:
        result, shift = 0, 0
        while True:
            b = self._read_byte()
            result |= (b & 0x7F) << shift
            shift += 7
            if not (b & 0x80):
                return result

    def read_bytes(self, n: int) -> bytes:
        out = self.data[self.pos: self.pos + n]
        self.pos += n
        return out

    def decode_all_fields(self) -> dict[int, list]:
        fields: dict[int, list] = {}
        while not self.done():
            tag = self.read_varint()
            field_number = tag >> 3
            wire_type = tag & 0x7
            if wire_type == 0:
                value = self.read_varint()
            elif wire_type == 1:
                value = self.read_bytes(8)
            elif wire_type == 2:
                length = self.read_varint()
                value = self.read_bytes(length)
            elif wire_type == 5:
                value = self.read_bytes(4)
            else:
                break  # unknown wire type — stop
            fields.setdefault(field_number, []).append(value)
        return fields


def _first(fields: dict, key: int, default=None):
    vals = fields.get(key, [])
    return vals[0] if vals else default


def _as_str(v) -> str | None:
    if isinstance(v, bytes):
        return v.decode("utf-8", errors="replace") or None
    return None


def _decode_proof_request(data: bytes) -> dict:
    """
    Decode a single ProofRequest message.

    Relevant field tags (sp1-sdk-6.1.0 auction/types.rs):
      3  = version          string
      4  = mode             ProofMode enum varint
      10 = gas_price        uint64 (atto-PROVE/PGU) — actual settled price, shown as "Price Per bPGU" in UI
      11 = fulfillment_status FulfillmentStatus enum varint
      13 = requester        bytes (address)
      18 = created_at       uint64 unix timestamp
      20 = fulfilled_at     uint64 unix timestamp (optional)
      27 = gas_used         uint64 (optional)
      34 = base_fee         string (atto-PROVE, optional)
      35 = max_price_per_pgu string (atto-PROVE/PGU, optional) — bidding max, not the settled price
    """
    dec = _Decoder(data)
    f = dec.decode_all_fields()

    request_id_bytes = _first(f, 1, b"")
    request_id = request_id_bytes.hex() if isinstance(request_id_bytes, bytes) else ""

    version = _as_str(_first(f, 3)) or ""
    mode_int = _first(f, 4, 0)
    mode_str = PROOF_MODE.get(mode_int, f"Unknown({mode_int})")

    status_int = _first(f, 11, 0)
    status_str = FULFILLMENT_STATUS.get(status_int, f"Unknown({status_int})")

    requester_bytes = _first(f, 13, b"")
    requester = ("0x" + requester_bytes.hex()) if isinstance(requester_bytes, bytes) else ""

    created_at = _first(f, 18, 0)
    fulfilled_at = _first(f, 20)  # optional

    time_taken_sec: int | None = None
    if fulfilled_at and created_at:
        time_taken_sec = int(fulfilled_at) - int(created_at)

    gas_used: int | None = _first(f, 27)
    # field 10 = gas_price (uint64, atto-PROVE/PGU): actual settled price shown as "Price Per bPGU"
    gas_price: int | None = _first(f, 10)
    base_fee: str | None = _as_str(_first(f, 34))

    # Unit conversions:
    #   base_fee (atto-PROVE)         → PROVE:       ÷ 1e18
    #   gas_price (atto-PROVE/PGU)    → PROVE/bPGU:  ÷ 1e18 × 1e9 = ÷ 1e9
    # total_fee (PROVE) = base_fee_PROVE + gas_used / 1e9 * price_per_bpgu_PROVE
    ATTO = 1_000_000_000_000_000_000  # 10^18

    base_fee_prove: float | None = None
    price_per_bpgu_prove: float | None = None
    total_fee_prove: float | None = None

    if base_fee is not None:
        try:
            base_fee_prove = float(base_fee) / ATTO
        except ValueError:
            pass
    if gas_price is not None:
        # atto-PROVE/PGU → PROVE/bPGU: multiply by 1e9 (PGU→bPGU), divide by 1e18 (atto→PROVE)
        price_per_bpgu_prove = gas_price / 1_000_000_000
    if gas_used is not None and base_fee_prove is not None and price_per_bpgu_prove is not None:
        total_fee_prove = base_fee_prove + gas_used / 1_000_000_000 * price_per_bpgu_prove

    created_iso = (
        datetime.fromtimestamp(created_at, tz=timezone.utc).isoformat()
        if created_at
        else None
    )
    fulfilled_iso = (
        datetime.fromtimestamp(fulfilled_at, tz=timezone.utc).isoformat()
        if fulfilled_at
        else None
    )

    return {
        "request_id": "0x" + request_id,
        "requester": requester,
        "status": status_str,
        "mode": mode_str,
        "sp1_version": version,
        "gas_used": gas_used,
        "time_taken_sec": time_taken_sec,
        # Fee fields — stored in PROVE (divided by 1e18 from raw atto-PROVE)
        "base_fee_prove": base_fee_prove,
        "price_per_bpgu_prove": price_per_bpgu_prove,
        "total_fee_prove": total_fee_prove,
        # Raw values from API
        "_raw_base_fee": base_fee,           # atto-PROVE string
        "_raw_gas_price": gas_price,         # atto-PROVE/PGU uint64 (settled price)
        "created_at": created_iso,
        "fulfilled_at": fulfilled_iso,
    }


def _decode_response(data: bytes) -> list[dict]:
    """
    Decode GetFilteredProofRequestsResponse.

    Field 1 = repeated ProofRequest (length-delimited).
    """
    dec = _Decoder(data)
    f = dec.decode_all_fields()
    results = []
    for req_bytes in f.get(1, []):
        if isinstance(req_bytes, bytes):
            results.append(_decode_proof_request(req_bytes))
    return results


# ---------------------------------------------------------------------------
# gRPC transport
# ---------------------------------------------------------------------------

def _parse_grpc_frames(raw: bytes) -> list[bytes]:
    """Parse one or more gRPC frames from a response body."""
    frames = []
    pos = 0
    while pos + 5 <= len(raw):
        compression_flag = raw[pos]
        msg_len = struct.unpack(">I", raw[pos + 1: pos + 5])[0]
        pos += 5
        if compression_flag & 0x80:
            # Trailers frame — stop
            break
        msg = raw[pos: pos + msg_len]
        pos += msg_len
        if compression_flag == 0:
            frames.append(msg)
    return frames


def fetch_fulfilled_proofs(page: int = 1, limit: int = 100) -> list[dict]:
    """
    Call GetFilteredProofRequests over gRPC (HTTP/2) and return decoded proof dicts.
    """
    pb_body = _encode_get_filtered_requests(
        fulfillment_status=3,  # FULFILLED
        limit=limit,
        page=page,
    )
    framed = _grpc_frame(pb_body)

    with httpx.Client(http2=True, timeout=30.0) as client:
        resp = client.post(
            f"{GRPC_ENDPOINT}/network.ProverNetwork/GetFilteredProofRequests",
            content=framed,
            headers={
                "Content-Type": "application/grpc",
                "TE": "trailers",
                "User-Agent": "succinct-proof-fetcher/1.0",
            },
        )

    grpc_status = resp.headers.get("grpc-status", "0")
    if grpc_status != "0":
        msg = resp.headers.get("grpc-message", "")
        raise RuntimeError(f"gRPC error {grpc_status}: {msg}")

    frames = _parse_grpc_frames(resp.content)
    proofs: list[dict] = []
    for frame in frames:
        proofs.extend(_decode_response(frame))
    return proofs


# ---------------------------------------------------------------------------
# Output formatters
# ---------------------------------------------------------------------------

def _fmt_time(seconds: int | None) -> str:
    if seconds is None:
        return "—"
    h, rem = divmod(seconds, 3600)
    m, s = divmod(rem, 60)
    if h:
        return f"{h}h {m}m {s}s"
    if m:
        return f"{m}m {s}s"
    return f"{s}s"


def _fmt_gas(gas: int | None) -> str:
    if gas is None:
        return "—"
    if gas >= 1_000_000_000_000:
        return f"{gas / 1e12:.3f}T"
    if gas >= 1_000_000_000:
        return f"{gas / 1e9:.3f}B"
    if gas >= 1_000_000:
        return f"{gas / 1e6:.3f}M"
    return str(gas)


def _fmt_prove(value: float | None, decimals: int = 6) -> str:
    if value is None:
        return "—"
    return f"{value:.{decimals}f} PROVE"


def print_markdown_table(proofs: list[dict]) -> None:
    headers = [
        "Request ID (short)",
        "Mode",
        "SP1 Version",
        "Gas Used",
        "Time Taken",
        "Price Per bPGU",
        "Base Fee",
        "Total Fee (est.)",
        "Fulfilled At",
    ]

    rows = []
    for p in proofs:
        rid = p["request_id"]
        short_id = rid[:10] + "..." + rid[-6:] if len(rid) > 18 else rid
        rows.append([
            short_id,
            p["mode"],
            p["sp1_version"] or "—",
            _fmt_gas(p["gas_used"]),
            _fmt_time(p["time_taken_sec"]),
            _fmt_prove(p["price_per_bpgu_prove"], decimals=10),
            _fmt_prove(p["base_fee_prove"], decimals=6),
            _fmt_prove(p["total_fee_prove"], decimals=6),
            p["fulfilled_at"] or "—",
        ])

    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            col_widths[i] = max(col_widths[i], len(str(cell)))

    def fmt_row(cells):
        return "| " + " | ".join(str(c).ljust(col_widths[i]) for i, c in enumerate(cells)) + " |"

    print(fmt_row(headers))
    print("| " + " | ".join("-" * w for w in col_widths) + " |")
    for row in rows:
        print(fmt_row(row))


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _parse_args():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--limit", type=int, default=100, help="Max proofs per page (1–100, default 100)")
    p.add_argument("--pages", type=int, default=1, help="Number of pages to fetch (default 1)")
    p.add_argument("--json", action="store_true", help="Output raw JSON instead of markdown table")
    p.add_argument("--mode", default="all", help="Filter by mode: all | plonk | groth16 | compressed | core")
    return p.parse_args()


def main():
    args = _parse_args()
    mode_filter = args.mode.lower()

    all_proofs: list[dict] = []

    for page in range(1, args.pages + 1):
        print(f"Fetching page {page} (limit={args.limit})…", file=sys.stderr)
        try:
            proofs = fetch_fulfilled_proofs(page=page, limit=args.limit)
        except Exception as e:
            print(f"Error on page {page}: {e}", file=sys.stderr)
            break

        print(f"  → {len(proofs)} fulfilled proofs", file=sys.stderr)
        if not proofs:
            break

        if mode_filter != "all":
            wanted = mode_filter.capitalize()
            proofs = [p for p in proofs if p["mode"].lower() == mode_filter]
            print(f"  → {len(proofs)} after mode filter ({wanted})", file=sys.stderr)

        proofs = [p for p in proofs if (_version_tuple(p.get("sp1_version") or "") or (0,)) >= (6,)]
        print(f"  → {len(proofs)} after version filter (>= sp1-v6)", file=sys.stderr)

        all_proofs.extend(proofs)

    print(f"\nTotal collected: {len(all_proofs)} proofs\n", file=sys.stderr)

    if args.json:
        print(json.dumps(all_proofs, indent=2))
    else:
        print_markdown_table(all_proofs)


if __name__ == "__main__":
    main()
