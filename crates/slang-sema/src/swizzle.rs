/// Vector component alphabets cannot be mixed in one selection.
pub(crate) fn vector(name: &str) -> Option<Vec<u8>> {
    if name.is_empty() || name.len() > 4 {
        return None;
    }
    for alphabet in ["xyzw", "rgba"] {
        if let Some(indices) = name
            .chars()
            .map(|ch| alphabet.find(ch).map(|i| i as u8))
            .collect::<Option<Vec<_>>>()
        {
            return Some(indices);
        }
    }
    None
}

/// Matrix selectors use either `_m00` (zero-based) or `_11` (one-based).
pub(crate) fn matrix(name: &str, rows: u8, columns: u8) -> Option<Vec<u8>> {
    let name = name.strip_prefix('_')?;
    let zero_based = name.starts_with('m');
    let mut indices = vec![];
    for component in name.split('_') {
        let component = if zero_based {
            component.strip_prefix('m')?
        } else {
            component
        };
        let [row, column] = component.as_bytes() else {
            return None;
        };
        let offset = if zero_based { b'0' } else { b'1' };
        let row = row.checked_sub(offset)?;
        let column = column.checked_sub(offset)?;
        if row >= rows || column >= columns || indices.len() == 4 {
            return None;
        }
        indices.push(row * columns + column);
    }
    Some(indices)
}

pub(crate) fn unique(indices: &[u8]) -> bool {
    indices
        .iter()
        .enumerate()
        .all(|(i, index)| !indices[..i].contains(index))
}
