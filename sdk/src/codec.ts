/**
 * Molecule codecs for CKB Vote types.
 * Mirrors crates/types/molecules/types.mol
 */

import { ccc } from "@ckb-ccc/shell";

const mol = ccc.mol;

/**
 * Codec that wraps ccc.Script for use as a nested field inside mol.table().
 * Matches the `Script` table type from blockchain.mol.
 */
export const ScriptMolCodec: ccc.mol.Codec<ccc.ScriptLike, ccc.Script> =
  mol.Codec.from({
    encode: (s: ccc.ScriptLike): ccc.Bytes => ccc.Script.encode(s),
    decode: (bytes: ccc.BytesLike): ccc.Script => ccc.Script.fromBytes(bytes),
  });

/**
 * table Vote {
 *   vote: byte,          // 0=NO, 1=YES
 *   amount: Uint64,
 *   dao_index: Uint16Vec,
 * }
 */
export const VoteCodec = mol.table({
  vote: mol.Uint8,
  amount: mol.Uint64,
  daoIndex: mol.Uint16Vec,
});

export type VoteEncodable = ccc.mol.EncodableType<typeof VoteCodec>;
export type VoteDecoded = ccc.mol.DecodedType<typeof VoteCodec>;

/**
 * table Proposal {
 *   duration: Uint32,
 *   vote_cell_code_hash: Byte32,
 *   vote_cell_hash_type: byte,
 *   description: Bytes,
 *   receiver: Script,
 *   amount: Uint64,
 *   minimal_requirement: Uint64,
 * }
 */
export const ProposalCodec = mol.table({
  duration: mol.Uint32,
  voteCellCodeHash: mol.Byte32,
  voteCellHashType: mol.Uint8,
  description: mol.Bytes,
  receiver: ScriptMolCodec,
  amount: mol.Uint64,
  minimalRequirement: mol.Uint64,
});

export type ProposalEncodable = ccc.mol.EncodableType<typeof ProposalCodec>;
export type ProposalDecoded = ccc.mol.DecodedType<typeof ProposalCodec>;

/**
 * table PublicValues {
 *   proposal: Proposal,
 *   start_block_hash: Byte32,
 *   end_block_hash: Byte32,
 *   proposal_script: Script,
 *   passed: byte,
 *   yes_vote: Uint64,
 *   no_vote: Uint64,
 * }
 */
export const PublicValuesCodec = mol.table({
  proposal: ProposalCodec,
  startBlockHash: mol.Byte32,
  endBlockHash: mol.Byte32,
  proposalScript: ScriptMolCodec,
  passed: mol.Uint8,
  yesVote: mol.Uint64,
  noVote: mol.Uint64,
});

export type PublicValuesEncodable = ccc.mol.EncodableType<
  typeof PublicValuesCodec
>;
export type PublicValuesDecoded = ccc.mol.DecodedType<typeof PublicValuesCodec>;

/**
 * table ProposalWitness {
 *   proof: Bytes,
 *   public_values: PublicValues,
 * }
 */
export const ProposalWitnessCodec = mol.table({
  proof: mol.Bytes,
  publicValues: PublicValuesCodec,
});

export type ProposalWitnessEncodable = ccc.mol.EncodableType<
  typeof ProposalWitnessCodec
>;
export type ProposalWitnessDecoded = ccc.mol.DecodedType<
  typeof ProposalWitnessCodec
>;
