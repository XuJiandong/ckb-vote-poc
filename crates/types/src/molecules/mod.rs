#![allow(dead_code)]
pub use ckb_gen_types::packed as blockchain;
pub mod types;

/// Verifies a serialized `BlockVec` slice.
///
/// Optimized verify: checks only the basic Block format, skipping transaction internals.
/// Transaction structure is already constrained by the transaction hash, so there is no security issue.
pub fn verify_block_vec(slice: &[u8], compatible: bool) -> molecule::error::VerificationResult<()> {
    use molecule::prelude::Reader as _;
    use molecule::verification_error as ve;
    use types::BlockVecReader;
    let slice_len = slice.len();
    if slice_len < molecule::NUMBER_SIZE {
        return ve!(
            BlockVecReader,
            HeaderIsBroken,
            molecule::NUMBER_SIZE,
            slice_len
        );
    }
    let total_size = molecule::unpack_number(slice) as usize;
    if slice_len != total_size {
        return ve!(BlockVecReader, TotalSizeNotMatch, total_size, slice_len);
    }
    if slice_len == molecule::NUMBER_SIZE {
        return Ok(());
    }
    if slice_len < molecule::NUMBER_SIZE * 2 {
        return ve!(
            BlockVecReader,
            TotalSizeNotMatch,
            molecule::NUMBER_SIZE * 2,
            slice_len
        );
    }
    let offset_first = molecule::unpack_number(&slice[molecule::NUMBER_SIZE..]) as usize;
    if offset_first % molecule::NUMBER_SIZE != 0 || offset_first < molecule::NUMBER_SIZE * 2 {
        return ve!(BlockVecReader, OffsetsNotMatch);
    }
    if slice_len < offset_first {
        return ve!(BlockVecReader, HeaderIsBroken, offset_first, slice_len);
    }
    let mut offsets: Vec<usize> = slice[molecule::NUMBER_SIZE..offset_first]
        .chunks_exact(molecule::NUMBER_SIZE)
        .map(|x| molecule::unpack_number(x) as usize)
        .collect();
    offsets.push(total_size);
    if offsets.windows(2).any(|i| i[0] > i[1]) {
        return ve!(BlockVecReader, OffsetsNotMatch);
    }
    for pair in offsets.windows(2) {
        let start = pair[0];
        let end = pair[1];
        verify_block_shallow(&slice[start..end], compatible)?;
    }
    Ok(())
}

pub fn verify_block_shallow(
    slice: &[u8],
    compatible: bool,
) -> molecule::error::VerificationResult<()> {
    use blockchain::ProposalShortIdVecReader;
    use molecule::prelude::Reader as _;
    use molecule::verification_error as ve;
    use types::BlockVecReader;
    let slice_len = slice.len();
    if slice_len < molecule::NUMBER_SIZE {
        return ve!(
            BlockVecReader,
            HeaderIsBroken,
            molecule::NUMBER_SIZE,
            slice_len
        );
    }
    let total_size = molecule::unpack_number(slice) as usize;
    if slice_len != total_size {
        return ve!(BlockVecReader, TotalSizeNotMatch, total_size, slice_len);
    }
    if slice_len < molecule::NUMBER_SIZE * 2 {
        return ve!(
            BlockVecReader,
            HeaderIsBroken,
            molecule::NUMBER_SIZE * 2,
            slice_len
        );
    }
    let offset_first = molecule::unpack_number(&slice[molecule::NUMBER_SIZE..]) as usize;
    if offset_first % molecule::NUMBER_SIZE != 0 || offset_first < molecule::NUMBER_SIZE * 2 {
        return ve!(BlockVecReader, OffsetsNotMatch);
    }
    if slice_len < offset_first {
        return ve!(BlockVecReader, HeaderIsBroken, offset_first, slice_len);
    }
    let field_count = offset_first / molecule::NUMBER_SIZE - 1;
    if field_count < 4 {
        return ve!(BlockVecReader, FieldCountNotMatch, 4, field_count);
    }
    // To support BlockV1
    // else if !compatible && field_count > 4 {
    //     return ve!(BlockVecReader, FieldCountNotMatch, 4, field_count);
    // };
    let mut offsets: Vec<usize> = slice[molecule::NUMBER_SIZE..offset_first]
        .chunks_exact(molecule::NUMBER_SIZE)
        .map(|x| molecule::unpack_number(x) as usize)
        .collect();
    offsets.push(total_size);
    if offsets.windows(2).any(|i| i[0] > i[1]) {
        return ve!(BlockVecReader, OffsetsNotMatch);
    }
    // offsets[0]..offsets[1] — Header struct (fixed 208 bytes)
    let header_slice = &slice[offsets[0]..offsets[1]];
    if header_slice.len() != 208 {
        return ve!(BlockVecReader, TotalSizeNotMatch, 208, header_slice.len());
    }
    // offsets[1]..offsets[2] — UncleBlockVec (vector; offset layout verified, element contents skipped)
    verify_vector_shallow(&slice[offsets[1]..offsets[2]])?;
    // offsets[2]..offsets[3] — TransactionVec (vector; offset layout verified, element contents skipped)
    verify_vector_shallow(&slice[offsets[2]..offsets[3]])?;
    // offsets[3]..offsets[4] — ProposalShortIdVec (fixed-size-item vector, fully verified by its own reader)
    ProposalShortIdVecReader::verify(&slice[offsets[3]..offsets[4]], compatible)?;
    Ok(())
}

pub fn verify_vector_shallow(slice: &[u8]) -> molecule::error::VerificationResult<()> {
    use molecule::prelude::Reader as _;
    use molecule::verification_error as ve;
    use types::BlockVecReader;
    let slice_len = slice.len();
    // Verify the vector has at least a 4-byte total_size header.
    if slice_len < molecule::NUMBER_SIZE {
        return ve!(
            BlockVecReader,
            HeaderIsBroken,
            molecule::NUMBER_SIZE,
            slice_len
        );
    }
    // Verify the total_size field matches the actual slice length.
    let total_size = molecule::unpack_number(slice) as usize;
    if slice_len != total_size {
        return ve!(BlockVecReader, TotalSizeNotMatch, total_size, slice_len);
    }
    // Empty vector (just the 4-byte total_size = 4): fully verified.
    if slice_len == molecule::NUMBER_SIZE {
        return Ok(());
    }
    // Non-empty vector requires at least 8 bytes (total_size + first offset).
    if slice_len < molecule::NUMBER_SIZE * 2 {
        return ve!(
            BlockVecReader,
            TotalSizeNotMatch,
            molecule::NUMBER_SIZE * 2,
            slice_len
        );
    }
    // Verify the first element offset: must be 4-byte-aligned and after the
    // header area (at least 8 bytes into the slice).
    let offset_first = molecule::unpack_number(&slice[molecule::NUMBER_SIZE..]) as usize;
    if offset_first % molecule::NUMBER_SIZE != 0 || offset_first < molecule::NUMBER_SIZE * 2 {
        return ve!(BlockVecReader, OffsetsNotMatch);
    }
    // Verify the first offset does not exceed total_size.
    if slice_len < offset_first {
        return ve!(BlockVecReader, HeaderIsBroken, offset_first, slice_len);
    }
    // Extract all element offsets from the offset area and verify they are
    // monotonically increasing (no overlapping or backward references).
    let mut offsets: Vec<usize> = slice[molecule::NUMBER_SIZE..offset_first]
        .chunks_exact(molecule::NUMBER_SIZE)
        .map(|x| molecule::unpack_number(x) as usize)
        .collect();
    offsets.push(total_size);
    if offsets.windows(2).any(|i| i[0] > i[1]) {
        return ve!(BlockVecReader, OffsetsNotMatch);
    }
    // Element contents are deliberately not verified — iteration via
    // get_unchecked / new_unchecked will still succeed as long as offsets
    // are valid.
    Ok(())
}
